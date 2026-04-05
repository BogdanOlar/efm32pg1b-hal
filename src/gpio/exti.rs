//! GPIO External Interrupts
//!
//! Bind an External Interrupt to a particular Pin.
//!
//! While any Exti can be assigned to any port, only some of the pins of that port can be bound to an Exti.
//!
//! |  ExtiId  |  Port  |      Pins      |
//! |----------|--------|----------------|
//! |     0    |   any  |  0,  1,  2,  3 |
//! |     1    |   any  |  0,  1,  2,  3 |
//! |     2    |   any  |  0,  1,  2,  3 |
//! |     3    |   any  |  0,  1,  2,  3 |
//! |     4    |   any  |  4,  5,  6,  7 |
//! |     5    |   any  |  4,  5,  6,  7 |
//! |     6    |   any  |  4,  5,  6,  7 |
//! |     7    |   any  |  4,  5,  6,  7 |
//! |     8    |   any  |  8,  9, 10, 11 |
//! |     9    |   any  |  8,  9, 10, 11 |
//! |    10    |   any  |  8,  9, 10, 11 |
//! |    11    |   any  |  8,  9, 10, 11 |
//! |    12    |   any  | 12, 13, 14, 15 |
//! |    13    |   any  | 12, 13, 14, 15 |
//! |    14    |   any  | 12, 13, 14, 15 |
//! |    15    |   any  | 12, 13, 14, 15 |
//!
//! In order to ensure that binding an Exti to a Pin is always valid, the user is expected to convert their pin and exti
//! into an [`ExtiBoundPin`].
//!
//! This ensures that:
//!     - Only enabled pins can be bound to and exti (i.e. that the pin mode is not `Disabled`, `DisabledPu` or `Analog`)
//!     - The mode of the pin does not change while it's bound to an exti
//!     - Only valid Pin-Exti bindings are allowed
//!
//! When creating an [`ExtiBoundPin`], the interrupt handler for the bound exti has to be provided. The HAL will take
//! care to call the apropriate handler whenever an Exti flag is raised, and it will also clear the Exti flag before
//! executing the handler.
//!
//! ```rust,norun
//! fn handler(exti: ExtiId) {
//!     // ...
//! }
//!
//! // Typestate Pin
//! let mut btn0 = gpio
//!     .pf6
//!     .into_mode::<InFloat>()
//!     .into_exti_bound_pin(gpio.exti4ctrl, handler);
//!
//! // Dynamic Pin
//! let mut btn1 = gpio
//!     .pf7
//!     .into_mode::<InFilt>()
//!     .into_dynamic_pin()
//!     .try_into_exti_bound_pin(gpio.exti5ctrl, handler)
//!     .unwrap();
//! ```
//! Or you can also use a closure
//! ```rust,norun
//! let mut btn0 = gpio
//!     .pf6
//!     .into_mode::<InFloat>()
//!     .into_exti_bound_pin(gpio.exti4ctrl, |exti| {
//!         // ...
//!     });
//! ```
//! Unfortunatelly, since the closure in the example above will be constrained to a function pointer, it cannot capture
//! any variables from its environment.
//!
//! Since this module provides a way for the user to run their code on specific external interrupts, both Gpio
//! interrupt vectors are implemented here, so you don't have to worry about EVEN or ODD external interrupts.
//!

use crate::{
    gpio::{
        dynamic::DynamicPin,
        pin::{mode::EnabledMode, PinInfo},
        GpioError, Pin,
    },
    pac::interrupt,
};
use core::cell::RefCell;
use critical_section::{CriticalSection, Mutex};

/// Number of External Interrupts
pub const EXTI_COUNT: usize = 16;

/// External interrupt Handler function.
/// Gets the ExtiId of the active external interrupt passed in as parameter
type ExtiHandler = fn(ExtiId);

/// Handler which does nothing
pub(crate) fn default_handler(_: ExtiId) {}

/// External interrupt handlers
static EXTI_HANDLERS: Mutex<RefCell<[ExtiHandler; EXTI_COUNT]>> =
    Mutex::new(RefCell::new([default_handler; _]));

/// Set the handler function for the given external interrupt
pub(crate) fn set_handler(cs: CriticalSection, exti: ExtiId, handler: ExtiHandler) {
    EXTI_HANDLERS.borrow(cs).borrow_mut()[exti as usize] = handler;
}

/// Handler which calls the user provided handlers in `EXTI_HANDLERS`
fn base_handler(exti: ExtiId) {
    // Clearing the interrupt flag _before_ executing the handler so that the handler can raise it back if needed
    mmio::exti_clear(exti);
    let handle = critical_section::with(|cs| EXTI_HANDLERS.borrow(cs).borrow()[exti as usize]);
    handle(exti);
}

#[interrupt]
fn GPIO_ODD() {
    for exti in mmio::exti_flags_odd() {
        base_handler(exti);
    }
}

#[interrupt]
fn GPIO_EVEN() {
    for exti in mmio::exti_flags_even() {
        base_handler(exti);
    }
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
    /// Convert the typestate `Pin` into an [`ExtiBoundPin`]
    pub fn into_exti_bound_pin<const GN: u8, const EN: u8>(
        self,
        mut exti_ctrl: ExtiCtrl<EN>,
        handler: ExtiHandler,
    ) -> ExtiBoundPin<Pin<P, N, MODE>, EN>
    where
        Pin<P, N, MODE>: PinInfo,
        // Only enabled pins may be bound to an external interrupt
        MODE: EnabledMode,
        // Only allow binding the Pin to the Exti if both are part of the same ExtiGroup
        ExtiCtrl<EN>: ExtiGroup<GN>,
        Self: ExtiGroup<GN>,
    {
        critical_section::with(|cs| {
            exti_ctrl.disable();
            exti_ctrl.clear();

            // It's safe to use `exti_bind_unchecked` here since the checks have been done by the type system.
            // i.e. an `AsyncInputPin` can only be created if both the `Pin` and `ExtiCtrl` implement the same
            // `ExtiGroup<GN>` trait ... therefore we know the pin can be bound to the external interrupt
            mmio::exti_bind_unchecked(exti_ctrl.id(), self.port(), self.pin());

            // Set the interrupt handler for async pins
            set_handler(cs, exti_ctrl.id(), handler);
        });

        ExtiBoundPin::new(self, exti_ctrl)
    }
}

impl DynamicPin {
    /// Try to convert the `DynamicPin` into an `ExtiBoundPin`
    ///
    /// This may fail if the pin is disabled, or if the Exti cannot be bound to this pin
    ///
    /// See [`crate::gpio::exti`] for more info on which pins can be bound to which external interrupts
    pub fn try_into_exti_bound_pin<const EN: u8>(
        self,
        mut exti_ctrl: ExtiCtrl<EN>,
        handler: ExtiHandler,
    ) -> Result<ExtiBoundPin<DynamicPin, EN>, GpioError> {
        if !mmio::exti_is_bind_valid(exti_ctrl.id(), self.pin()) {
            Err(GpioError::InvalidExiBind {
                exti: exti_ctrl.id(),
                port: self.port(),
                pin: self.pin(),
            })
        } else if !self.mode().readable() {
            // Pin is Disabled or in Analog mode
            Err(GpioError::InvalidMode(self.mode()))
        } else {
            critical_section::with(|cs| {
                exti_ctrl.disable();
                exti_ctrl.clear();
                mmio::exti_bind_unchecked(exti_ctrl.id(), self.port(), self.pin());
                // Set the interrupt handler for async pins
                set_handler(cs, exti_ctrl.id(), handler);
            });

            Ok(ExtiBoundPin::new(self, exti_ctrl))
        }
    }
}

/// A `PIN` which is bound to an external interrupt `ExtiCtrl<EN>`
///
/// Use the provided methods to get references to the original `PIN` and `ExtiCtrl`
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExtiBoundPin<PIN, const EN: u8> {
    pin: PIN,
    exti_ctrl: ExtiCtrl<EN>,
}

impl<PIN, const EN: u8> ExtiBoundPin<PIN, EN> {
    pub(crate) fn new(pin: PIN, exti_ctrl: ExtiCtrl<EN>) -> Self {
        Self { pin, exti_ctrl }
    }

    /// Get a reference to the original pin
    pub fn pin_ref(&self) -> &PIN {
        &self.pin
    }

    /// Get a mutable reference to the original pin
    pub fn pin_ref_mut(&mut self) -> &mut PIN {
        &mut self.pin
    }

    /// Get a reference to the original external interrupt controller
    pub fn exti_ctrl_ref(&self) -> &ExtiCtrl<EN> {
        &self.exti_ctrl
    }

    /// Get a mutable reference to the original external interrupt controller
    pub fn exti_ctrl_ref_mut(&mut self) -> &mut ExtiCtrl<EN> {
        &mut self.exti_ctrl
    }

    /// Return the PIN and `ExtiCtrl` used to construct this `ExtiBoundPin`
    pub fn release(self) -> (PIN, ExtiCtrl<EN>) {
        // cleanup
        critical_section::with(|cs| {
            mmio::exti_disable(self.exti_ctrl.id());
            mmio::exti_clear(self.exti_ctrl.id());
            mmio::exti_edge_clear(self.exti_ctrl.id(), ExtiEdge::Both);
            set_handler(cs, self.exti_ctrl.id(), default_handler);
        });

        (self.pin, self.exti_ctrl)
    }
}

/// Controller for External Interrupt `EN`
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExtiCtrl<const EN: u8> {
    /// Ensure only this crate can instantiate `ExtiCtrl` (see [`ExtiCtrl::new`])
    _p: (),
}

impl<const EN: u8> ExtiCtrl<EN> {
    /// Create the controller for External Interrupt `N`
    pub(crate) fn new() -> Self {
        Self { _p: () }
    }

    /// Get the External Interrupt ID from this controller
    pub fn id(&self) -> ExtiId {
        ExtiId::from_u8_unchecked(EN)
    }

    /// Sellect the interrupt edge for this Exti
    pub fn edge_select(&mut self, edge: ExtiEdge) {
        mmio::exti_edge_select(self.id(), edge);
    }

    /// Enable interrupts for this Exti
    pub fn enable(&mut self) {
        mmio::exti_enable(self.id());
    }

    /// Disable interrupts for this Exti
    pub fn disable(&mut self) {
        mmio::exti_disable(self.id());
    }

    /// Clear interrupt flag for this Exti
    pub fn clear(&mut self) {
        mmio::exti_clear(self.id());
    }
}

/// This trait is used to map which Exti can be bound to which typestate Pin
///
/// A pin and an external interrupt can only be bound if they both implement the same ExtiGroup<GN>
pub trait ExtiGroup<const GN: u8> {}

impl ExtiGroup<0> for ExtiCtrl<0> {}
impl ExtiGroup<0> for ExtiCtrl<1> {}
impl ExtiGroup<0> for ExtiCtrl<2> {}
impl ExtiGroup<0> for ExtiCtrl<3> {}
impl ExtiGroup<1> for ExtiCtrl<4> {}
impl ExtiGroup<1> for ExtiCtrl<5> {}
impl ExtiGroup<1> for ExtiCtrl<6> {}
impl ExtiGroup<1> for ExtiCtrl<7> {}
impl ExtiGroup<2> for ExtiCtrl<8> {}
impl ExtiGroup<2> for ExtiCtrl<9> {}
impl ExtiGroup<2> for ExtiCtrl<10> {}
impl ExtiGroup<2> for ExtiCtrl<11> {}
impl ExtiGroup<3> for ExtiCtrl<12> {}
impl ExtiGroup<3> for ExtiCtrl<13> {}
impl ExtiGroup<3> for ExtiCtrl<14> {}
impl ExtiGroup<3> for ExtiCtrl<15> {}

impl<const P: char, MODE> ExtiGroup<0> for Pin<P, 0, MODE> {}
impl<const P: char, MODE> ExtiGroup<0> for Pin<P, 1, MODE> {}
impl<const P: char, MODE> ExtiGroup<0> for Pin<P, 2, MODE> {}
impl<const P: char, MODE> ExtiGroup<0> for Pin<P, 3, MODE> {}
impl<const P: char, MODE> ExtiGroup<1> for Pin<P, 4, MODE> {}
impl<const P: char, MODE> ExtiGroup<1> for Pin<P, 5, MODE> {}
impl<const P: char, MODE> ExtiGroup<1> for Pin<P, 6, MODE> {}
impl<const P: char, MODE> ExtiGroup<1> for Pin<P, 7, MODE> {}
impl<const P: char, MODE> ExtiGroup<2> for Pin<P, 8, MODE> {}
impl<const P: char, MODE> ExtiGroup<2> for Pin<P, 9, MODE> {}
impl<const P: char, MODE> ExtiGroup<2> for Pin<P, 10, MODE> {}
impl<const P: char, MODE> ExtiGroup<2> for Pin<P, 11, MODE> {}
impl<const P: char, MODE> ExtiGroup<3> for Pin<P, 12, MODE> {}
impl<const P: char, MODE> ExtiGroup<3> for Pin<P, 13, MODE> {}
impl<const P: char, MODE> ExtiGroup<3> for Pin<P, 14, MODE> {}
impl<const P: char, MODE> ExtiGroup<3> for Pin<P, 15, MODE> {}

/// External Interrupt ID
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ExtiId {
    /// External Interrupt 0
    Exti0,
    /// External Interrupt 1
    Exti1,
    /// External Interrupt 2
    Exti2,
    /// External Interrupt 3
    Exti3,
    /// External Interrupt 4
    Exti4,
    /// External Interrupt 5
    Exti5,
    /// External Interrupt 6
    Exti6,
    /// External Interrupt 7
    Exti7,
    /// External Interrupt 8
    Exti8,
    /// External Interrupt 9
    Exti9,
    /// External Interrupt 10
    Exti10,
    /// External Interrupt 11
    Exti11,
    /// External Interrupt 12
    Exti12,
    /// External Interrupt 13
    Exti13,
    /// External Interrupt 14
    Exti14,
    /// External Interrupt 15
    Exti15,
}

impl ExtiId {
    pub(crate) const fn from_u8_unchecked(e: u8) -> Self {
        match e & 0b1111 {
            0 => Self::Exti0,
            1 => Self::Exti1,
            2 => Self::Exti2,
            3 => Self::Exti3,
            4 => Self::Exti4,
            5 => Self::Exti5,
            6 => Self::Exti6,
            7 => Self::Exti7,
            8 => Self::Exti8,
            9 => Self::Exti9,
            10 => Self::Exti10,
            11 => Self::Exti11,
            12 => Self::Exti12,
            13 => Self::Exti13,
            14 => Self::Exti14,
            15 => Self::Exti15,
            _ => unreachable!(),
        }
    }
}

impl TryFrom<u8> for ExtiId {
    type Error = GpioError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Exti0),
            1 => Ok(Self::Exti1),
            2 => Ok(Self::Exti2),
            3 => Ok(Self::Exti3),
            4 => Ok(Self::Exti4),
            5 => Ok(Self::Exti5),
            6 => Ok(Self::Exti6),
            7 => Ok(Self::Exti7),
            8 => Ok(Self::Exti8),
            9 => Ok(Self::Exti9),
            10 => Ok(Self::Exti10),
            11 => Ok(Self::Exti11),
            12 => Ok(Self::Exti12),
            13 => Ok(Self::Exti13),
            14 => Ok(Self::Exti14),
            15 => Ok(Self::Exti15),
            _ => Err(GpioError::InvalidExiValue(value)),
        }
    }
}

/// External Interrupt Edge
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ExtiEdge {
    /// Rising edge
    Rising,
    /// Falling edge
    Falling,
    /// Both Rising and Falling edges
    Both,
}

/// Access functions for external interrupts Memory Mapped IO
pub mod mmio {
    use crate::{
        gpio::{
            exti::{ExtiEdge, ExtiId},
            pin::PinId,
            port::PortId,
        },
        pac::Gpio,
    };

    const SEL_GROUP_SIZE: u8 = 4;
    /// Selection groups per register (either High or Low versions of `EXTIPSEL` and `EXTIPINSEL` registers)
    const SEL_GROUPS: u8 = u32::BITS as u8 / SEL_GROUP_SIZE;
    const SEL_PORT_BIT_MASK: u32 = 0x0F;
    const SEL_PIN_BIT_MASK: u32 = 0b11;

    pub(crate) fn exti_bind_unchecked(exti: ExtiId, port: PortId, pin: PinId) {
        let gpio = gpio();
        let offset = (exti as u8 % SEL_GROUPS) * SEL_GROUP_SIZE;
        let port_reg_val = (port as u32) << offset;
        let port_mask = SEL_PORT_BIT_MASK << offset;
        let pin_reg_val = ((pin as u32) % SEL_GROUP_SIZE as u32) << offset;
        let pin_mask = SEL_PIN_BIT_MASK << offset;

        if exti_is_low_reg(exti) {
            gpio.extipsell()
                .modify(|r, w| unsafe { w.bits((r.bits() & !port_mask) | port_reg_val) });
            gpio.extipinsell()
                .modify(|r, w| unsafe { w.bits((r.bits() & !pin_mask) | pin_reg_val) });
        } else {
            gpio.extipselh()
                .modify(|r, w| unsafe { w.bits((r.bits() & !port_mask) | port_reg_val) });
            gpio.extipinselh()
                .modify(|r, w| unsafe { w.bits((r.bits() & !pin_mask) | pin_reg_val) });
        }
    }

    /// Get the pin binding of the given external interrupt ID
    pub fn exti_bind_get(exti: ExtiId) -> (PortId, PinId) {
        let gpio = gpio();

        let pin_base_id = (exti as u8 / SEL_GROUP_SIZE) * SEL_GROUP_SIZE;
        let offset = (exti as u8 % SEL_GROUPS) * SEL_GROUP_SIZE;
        let port_mask = SEL_PORT_BIT_MASK << offset;
        let pin_mask = SEL_PIN_BIT_MASK << offset;

        let (port_reg_val, pin_reg_val) = match exti_is_low_reg(exti) {
            true => (
                gpio.extipsell().read().bits(),
                gpio.extipinsell().read().bits(),
            ),
            false => (
                gpio.extipselh().read().bits(),
                gpio.extipinselh().read().bits(),
            ),
        };
        let port = ((port_reg_val & port_mask) >> offset) as u8;
        let pin = ((pin_reg_val & pin_mask) >> offset) as u8 + pin_base_id;

        (
            PortId::from_u8_unchecked(port),
            PinId::from_u8_unchecked(pin),
        )
    }

    /// Enable given external interrupt
    pub fn exti_enable(exti: ExtiId) {
        gpio()
            .ien()
            .modify(|r, w| unsafe { w.ext().bits(r.ext().bits() | 1 << exti as u8) });
    }

    /// Check if external interrupt is enabled
    pub fn exti_is_enabled(exti: ExtiId) -> bool {
        gpio().ien().read().ext().bits() & 1 << exti as u8 != 0
    }

    /// Disable given external interrupt
    pub fn exti_disable(exti: ExtiId) {
        gpio()
            .ien()
            .modify(|r, w| unsafe { w.ext().bits(r.ext().bits() & !(1 << exti as u8)) });
    }

    /// Checl if the interrupt flag is raised for the given external interrupt
    pub fn exti_get(exti: ExtiId) -> bool {
        (gpio().if_().read().ext().bits() & (1 << (exti as u8))) != 0
    }

    /// Iterator over all raised EVEN external interrupt flags
    pub(crate) fn exti_flags_even() -> impl Iterator<Item = ExtiId> {
        let exti_cached_flags = gpio().if_().read().ext().bits();

        (ExtiId::Exti0 as u8..=ExtiId::Exti14 as u8)
            .step_by(2)
            .filter(move |i| ((1 << *i) & exti_cached_flags) != 0)
            .map(ExtiId::from_u8_unchecked)
    }

    /// Iterator over all raised ODD external interrupt flags
    pub(crate) fn exti_flags_odd() -> impl Iterator<Item = ExtiId> {
        let exti_cached_flags = gpio().if_().read().ext().bits();

        (ExtiId::Exti1 as u8..=ExtiId::Exti15 as u8)
            .step_by(2)
            .filter(move |i| ((1 << *i) & exti_cached_flags) != 0)
            .map(ExtiId::from_u8_unchecked)
    }

    /// Clear external interrupt flag
    pub fn exti_clear(exti: ExtiId) {
        gpio()
            .ifc()
            .write(|w| unsafe { w.ext().bits(1 << (exti as u8)) });
    }

    /// Select the edge which triggers the external interrupt
    pub fn exti_edge_select(exti: ExtiId, edge: ExtiEdge) {
        let gpio = gpio();
        let exti_mask = 1 << exti as u8;

        match edge {
            ExtiEdge::Rising => {
                gpio.extirise()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
                gpio.extifall()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !exti_mask) });
            }
            ExtiEdge::Falling => {
                gpio.extirise()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !exti_mask) });
                gpio.extifall()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
            }
            ExtiEdge::Both => {
                gpio.extirise()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
                gpio.extifall()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
            }
        };
    }

    /// Get the edge(s) which trigger the external interrupt
    pub fn exti_edge_get(exti: ExtiId) -> Option<ExtiEdge> {
        let gpio = gpio();
        let exti_mask = 1 << exti as u8;

        let rising = (gpio.extirise().read().bits() & exti_mask) != 0;
        let falling = (gpio.extifall().read().bits() & exti_mask) != 0;

        if rising && falling {
            Some(ExtiEdge::Both)
        } else if rising {
            Some(ExtiEdge::Rising)
        } else if falling {
            Some(ExtiEdge::Falling)
        } else {
            None
        }
    }

    /// Clear the edge(s) which trigger the external interrupt
    pub fn exti_edge_clear(exti: ExtiId, edge: ExtiEdge) {
        let gpio = gpio();
        let exti_mask = !(1 << exti as u8);

        if edge == ExtiEdge::Rising || edge == ExtiEdge::Both {
            gpio.extirise()
                .modify(|r, w| unsafe { w.bits(r.bits() & exti_mask) });
        }

        if edge == ExtiEdge::Falling || edge == ExtiEdge::Both {
            gpio.extifall()
                .modify(|r, w| unsafe { w.bits(r.bits() & exti_mask) });
        }
    }

    /// Enable energy mode 4 wake up for the given external interrupt
    pub fn exti_enable_em4wu(exti: ExtiId) {
        gpio()
            .ien()
            .modify(|_, w| unsafe { w.em4wu().bits(1 << exti as u8) });
    }

    /// Check if given Pin can be bound to given Exti
    pub const fn exti_is_bind_valid(exti: ExtiId, pin: PinId) -> bool {
        let exti_group = exti as u8 / SEL_GROUP_SIZE;
        let pin_group = pin as u8 / SEL_GROUP_SIZE;
        exti_group == pin_group
    }

    const fn exti_is_low_reg(exti: ExtiId) -> bool {
        (exti as u8) < (ExtiId::Exti8 as u8)
    }

    #[inline(always)]
    fn gpio() -> Gpio {
        unsafe { crate::pac::Gpio::steal() }
    }
}
