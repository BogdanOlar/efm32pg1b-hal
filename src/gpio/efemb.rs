//! Embassy GPIO support
//!

use core::{fmt::Debug, future::Future, task::Poll};

use crate::gpio::{
    dynamic::DynamicPin,
    exti::{self, ExtiCtrl, ExtiEdge, ExtiGroup, ExtiId, EXTI_COUNT},
    pin::{mode::MultiMode, PinInfo},
    GpioError, Pin,
};
use embassy_sync::waitqueue::AtomicWaker;
use embedded_hal::digital::{ErrorType, InputPin};
use embedded_hal_async::digital::Wait;

/// Embassy task wakers for each external interrupt
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
        mut exti_ctrl: ExtiCtrl<EN>,
    ) -> AsyncInputPin
    where
        ExtiCtrl<EN>: ExtiGroup<GN>,
        Self: ExtiGroup<GN>,
    {
        critical_section::with(|cs| {
            exti_ctrl.disable();
            exti_ctrl.clear();

            // It's safe to use `exti_bind_unchecked` here since the checks have been done by the type system.
            // i.e. an `AsyncInputPin` can only be created if both the `Pin` and `ExtiCtrl` implement the same
            // `ExtiGroup<GN>` trait ... therefore we know the pin can be bound to the external interrupt
            exti::mmio::exti_bind_unchecked(exti_ctrl.id(), self.port(), self.pin());

            // Set the interrupt handler for async pins
            exti::set_handler(cs, exti_ctrl.id(), on_interrupt);
        });
        AsyncInputPin::new(
            DynamicPin::new(self.port(), self.pin(), MODE::dynamic_mode()),
            exti_ctrl.id(),
        )
    }
}

impl DynamicPin {
    /// Try to Convert the `DynamicPin` into an `AsyncInputPin`
    ///
    /// This may fail if the pin is not an input pin, or if the Exti cannot be bound to this pin
    ///
    /// See [`crate::gpio::exti`] for more info on which pins can be bound to which external interrupts
    pub fn try_into_async_input<const N: u8>(
        self,
        mut exti_ctrl: ExtiCtrl<N>,
    ) -> Result<AsyncInputPin, GpioError> {
        if !exti::mmio::exti_is_bind_valid(exti_ctrl.id(), self.pin()) {
            Err(GpioError::InvalidExiBind {
                exti: exti_ctrl.id(),
                port: self.port(),
                pin: self.pin(),
            })
        } else if !self.mode().readable_input() {
            Err(GpioError::InvalidMode(self.mode()))
        } else {
            critical_section::with(|cs| {
                exti_ctrl.disable();
                exti_ctrl.clear();
                exti::mmio::exti_bind_unchecked(exti_ctrl.id(), self.port(), self.pin());
                // Set the interrupt handler for async pins
                exti::set_handler(cs, exti_ctrl.id(), on_interrupt);
            });

            Ok(AsyncInputPin::new(self, exti_ctrl.id()))
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
        // cleanup
        critical_section::with(|cs| {
            exti::mmio::exti_disable(self.exti);
            exti::mmio::exti_clear(self.exti);
            exti::mmio::exti_edge_clear(self.exti, ExtiEdge::Both);
            exti::set_handler(cs, self.exti, exti::default_handler);
        });

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
    type Output = Result<(), GpioError>;

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

impl ErrorType for AsyncInputPin {
    type Error = GpioError;
}
