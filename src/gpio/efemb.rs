//! Embassy GPIO support
//!

use core::{fmt::Debug, future::Future, task::Poll};

use crate::{
    gpio::{
        dynamic::{DynamicPin, PinMode},
        exti::{self, ExtiCtrl, ExtiEdge, ExtiGroup, ExtiId, EXTI_COUNT},
        pin::{mode::MultiMode, PinId, PinInfo},
        Pin,
    },
    pac::interrupt,
};
use embassy_sync::waitqueue::AtomicWaker;
use embedded_hal::digital::{Error, ErrorType, InputPin};
use embedded_hal_async::digital::Wait;

static EXTI_WAKERS: [AtomicWaker; EXTI_COUNT] = [const { AtomicWaker::new() }; EXTI_COUNT];

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE>
where
    Pin<P, N, MODE>: InputPin,
    MODE: MultiMode,
{
    /// Convert the typestate `Pin` into an `AsyncInputPin`
    ///
    /// WARNING: this will type-erase the `Pin` and the `ExtiCtrl` because most ofen these pins need to be provided to
    ///          Embassy tasks as parameters, which is not possible with generic types
    pub fn into_async_input<const GN: u8, const EN: u8>(
        self,
        exti_ctrl: ExtiCtrl<EN>,
    ) -> AsyncInputPin
    where
        ExtiCtrl<EN>: ExtiGroup<GN>,
        Self: ExtiGroup<GN>,
    {
        // It's safe to use `exti_bind_unchecked` here since the checks have been done by the type system.
        // i.e. an `AsyncInputPin` can only be created if both the `Pin` and `ExtiCtrl` implement the same
        // `ExtiGroup<GN>` trait
        exti::mmio::exti_bind_unchecked(exti_ctrl.id(), self.port(), self.pin());

        AsyncInputPin::new(
            DynamicPin::new(self.port(), self.pin(), MODE::dynamic_mode()),
            exti_ctrl.id(),
        )
    }
}

impl DynamicPin {
    /// Try to Convert the `DynamicPin` into an `AsyncInputPin`
    ///
    /// This may fail if the pin is not an input pin, or if the Exti cannot be bound to this `DynamicPin`
    ///
    /// See [`crate::gpio::exti::mmio::exti_bind`] for more info on which pins can be bound to which external interrupts
    pub fn try_into_async_input<const N: u8>(
        self,
        exti_ctrl: ExtiCtrl<N>,
    ) -> Result<AsyncInputPin, AsyncInputError> {
        exti::mmio::exti_bind(exti_ctrl.id(), self.port(), self.pin())
            .map_err(|_e| AsyncInputError::InvalidExiBind(self.pin(), exti_ctrl.id()))?;

        if self.mode().readable_input() {
            Ok(AsyncInputPin::new(self, exti_ctrl.id()))
        } else {
            Err(AsyncInputError::InvalidPinMode(self.mode()))
        }
    }
}

/// Input pin which can be used with async tasks
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AsyncInputPin {
    pin: DynamicPin,
    exti: ExtiId,
}

impl AsyncInputPin {
    pub(crate) fn new(pin: DynamicPin, exti: ExtiId) -> Self {
        Self { pin, exti }
    }

    /// Return the [`Pin`] and [`ExtiCtrl`] used to create this `AsyncInputPin`
    ///
    /// The [`ExtiCtrl`] will be disabled before being released
    ///
    /// FIXME: implement a runtime type for `ExtiCtrl`, since returning the Id is not very useful
    pub fn release(self) -> (DynamicPin, ExtiId) {
        exti::mmio::exti_disable(self.exti);
        (self.pin, self.exti)
    }
}

impl Wait for AsyncInputPin {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        if self.pin.is_high().unwrap() {
            Ok(())
        } else {
            ExtiFuture::new(self.exti, ExtiEdge::Rising).await
        }
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        if self.pin.is_low().unwrap() {
            Ok(())
        } else {
            ExtiFuture::new(self.exti, ExtiEdge::Falling).await
        }
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        ExtiFuture::new(self.exti, ExtiEdge::Rising).await
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        ExtiFuture::new(self.exti, ExtiEdge::Falling).await
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        ExtiFuture::new(self.exti, ExtiEdge::Both).await
    }
}

struct ExtiFuture {
    exti: ExtiId,
}

impl ExtiFuture {
    /// Create a new Future which awaits for the given [`ExtiEdge`]
    ///
    /// Note: this assumes the corresponding Exti is disabled at the moment where the [`ExtiFuture`] is created
    pub(crate) fn new(exti: ExtiId, edge: ExtiEdge) -> Self {
        exti::mmio::exti_clear(exti);
        exti::mmio::exti_edge_select(exti, edge);
        exti::mmio::exti_enable(exti);
        Self { exti }
    }
}

impl Future for ExtiFuture {
    type Output = Result<(), AsyncInputError>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        EXTI_WAKERS[self.exti as usize].register(cx.waker());

        // We are using the edge select bits as signal to the task that the interrupt occurred: the bits have been set to
        // _something_ when the future was created, and are cleared to nothing when the corresponding exti interrupt ran
        if exti::mmio::exti_edge_get(self.exti).is_none() {
            // The interrupt should already be disabled and cleared by the `on_interrupt()` handler, so we're not doing
            // that again here

            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

impl Drop for ExtiFuture {
    fn drop(&mut self) {
        // Make sure the external interrupt is cleaned up
        exti::mmio::exti_disable(self.exti);
        exti::mmio::exti_clear(self.exti);
        exti::mmio::exti_edge_clear(self.exti, ExtiEdge::Both);
    }
}

/// Interrupt handler for both Even and Odd interrupt vectors
fn on_interrupt(exti: ExtiId) {
    EXTI_WAKERS[exti as usize].wake();
    exti::mmio::exti_disable(exti);
    exti::mmio::exti_clear(exti);
    // We are clearing the edge select bits to signal to the task that the interrupt occurred
    exti::mmio::exti_edge_clear(exti, ExtiEdge::Both);
}

#[interrupt]
fn GPIO_ODD() {
    for exti in exti::mmio::exti_flags_odd() {
        on_interrupt(exti);
    }
}

#[interrupt]
fn GPIO_EVEN() {
    for exti in exti::mmio::exti_flags_even() {
        on_interrupt(exti);
    }
}

/// Async Input Errors
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AsyncInputError {
    /// The pin cannot be bound to the external interrupt
    /// See [`crate::gpio::exti::mmio::exti_bind`] for more info on which pins can be bound to which external interrupts
    InvalidExiBind(PinId, ExtiId),
    /// The pin is not an input pin
    InvalidPinMode(PinMode),
}

impl Error for AsyncInputError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        match self {
            Self::InvalidExiBind(_, _) => embedded_hal::digital::ErrorKind::Other,
            Self::InvalidPinMode(..) => embedded_hal::digital::ErrorKind::Other,
        }
    }
}

impl ErrorType for AsyncInputPin {
    type Error = AsyncInputError;
}
