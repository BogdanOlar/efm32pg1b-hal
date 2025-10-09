//! Zero-sized pins
//!
//!

use crate::{
    gpio::{
        dynamic::DynamicPin,
        erased::ErasedPin,
        pin::mode::{InputMode, MultiMode, Out, OutAlt, OutputMode},
        port::{self, PortId},
        GpioError,
    },
    Sealed,
};
use core::{fmt, marker::PhantomData};
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

/// Generic pin type
///
/// - `MODE` is one of the pin modes (see [Modes](crate::gpio#modes) section).
/// - `P` is port name: `A` for GPIOA, `B` for GPIOB, etc.
/// - `N` is pin number: from `0` to `15`.
pub struct Pin<const P: char, const N: u8, MODE> {
    _mode: PhantomData<MODE>,
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
    pub(crate) const fn new() -> Self {
        Self { _mode: PhantomData }
    }
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE>
where
    MODE: MultiMode + Sealed,
    Pin<P, N, MODE>: Sealed,
{
    /// Transition a pin from one mode to another. Available modes (see also [`crate::gpio#modes`] details):
    ///
    /// * _Disabled_:
    ///   [`Disabled`](`pin::mode::Disabled`),
    ///   [`DisabledPu`](`pin::mode::DisabledPu`),
    ///   [`Analog`](`pin::mode::Analog`)
    ///
    /// * _Input_:
    ///   [`InFloat`](`pin::mode::InFloat`),
    ///   [`InFilt`](`pin::mode::InFilt`),
    ///   [`InPu`](`pin::mode::InPu`),
    ///   [`InPuFilt`](`pin::mode::InPuFilt`),
    ///   [`InPd`](`pin::mode::InPd`),
    ///   [`InPdFilt`](`pin::mode::InPdFilt`)
    ///
    /// * _Output_:
    ///   [`OutPp`](`pin::mode::OutPp`),
    ///   [`OutOs`](`pin::mode::OutOs`),
    ///   [`OutOsPd`](`pin::mode::OutOsPd`),
    ///   [`OutOd`](`pin::mode::OutOd`),
    ///   [`OutOd`](`pin::mode::OutOdFilt`),
    ///   [`OutOdPu`](`pin::mode::OutOdPu`),
    ///   [`OutOdPuFilt`](`pin::mode::OutOdPuFilt`)
    ///
    /// * _Alternate Output_:
    ///   [`OutPpAlt`](`pin::mode::OutPpAlt`),
    ///   [`OutOdAlt`](`pin::mode::OutOdAlt`),
    ///   [`OutOdFiltAlt`](`pin::mode::OutOdFiltAlt`),
    ///   [`OutOdPuAlt`](`pin::mode::OutOdPuAlt`),
    ///   [`OutOdPuFiltAlt`](`pin::mode::OutOdPuFiltAlt`)
    ///
    /// Example
    ///
    /// ```rust,no_run
    ///     // create an input pin with filter
    ///     let mut btn0 = gpio.pf6.into_mode::<InFilt>();
    ///     // convert the pin into an Alternative Push-Pull Output pin
    ///     let mut led0 = btn0.into_mode::<OutPpAlt>();
    /// ```
    pub fn into_mode<NMODE>(self) -> Pin<P, N, NMODE>
    where
        NMODE: MultiMode + Sealed,
        Pin<P, N, NMODE>: Sealed,
    {
        NMODE::set_regs(self.port(), self.pin());
        Pin::new()
    }

    /// Temporarily set the mode of a given pin to a new mode while executing the given closure `f`.
    /// Available modes (see also [`crate::gpio#modes`] details):
    ///
    /// * _Disabled_:
    ///   [`Disabled`](`pin::mode::Disabled`),
    ///   [`DisabledPu`](`pin::mode::DisabledPu`),
    ///   [`Analog`](`pin::mode::Analog`)
    ///
    /// * _Input_:
    ///   [`InFloat`](`pin::mode::InFloat`),
    ///   [`InFilt`](`pin::mode::InFilt`),
    ///   [`InPu`](`pin::mode::InPu`),
    ///   [`InPuFilt`](`pin::mode::InPuFilt`),
    ///   [`InPd`](`pin::mode::InPd`),
    ///   [`InPdFilt`](`pin::mode::InPdFilt`)
    ///
    /// * _Output_:
    ///   [`OutPp`](`pin::mode::OutPp`),
    ///   [`OutOs`](`pin::mode::OutOs`),
    ///   [`OutOsPd`](`pin::mode::OutOsPd`),
    ///   [`OutOd`](`pin::mode::OutOd`),
    ///   [`OutOd`](`pin::mode::OutOdFilt`),
    ///   [`OutOdPu`](`pin::mode::OutOdPu`),
    ///   [`OutOdPuFilt`](`pin::mode::OutOdPuFilt`)
    ///
    /// * _Alternate Output_:
    ///   [`OutPpAlt`](`pin::mode::OutPpAlt`),
    ///   [`OutOdAlt`](`pin::mode::OutOdAlt`),
    ///   [`OutOdFiltAlt`](`pin::mode::OutOdFiltAlt`),
    ///   [`OutOdPuAlt`](`pin::mode::OutOdPuAlt`),
    ///   [`OutOdPuFiltAlt`](`pin::mode::OutOdPuFiltAlt`)
    ///
    /// Example:
    ///
    /// ```rust,no_run
    ///     // temporarily convert pin A0 from Disabled (default) to an input pin with PULL-UP enabled
    ///     let state_result = gpio.pa0.with_mode::<InPu, _>(|pin| pin.is_high());
    ///     assert!(state_result.is_ok_and(|pin_is_high| pin_is_high));
    ///
    ///     // pin A0 is again disabled
    ///
    ///     // temporarily convert pin A0 from Disabled (default) to an input pin with PULL-DOWN enabled
    ///     let state_result = gpio.pa0.with_mode::<InPd, _>(|pin| pin.is_high());
    ///     assert!(state_result.is_ok_and(|pin_is_high| !pin_is_high));
    ///
    ///     // pin A0 is again disabled
    /// ```
    /// Note that the return type `R` can be omitted with `_`, since it will be automatically deduced based on the
    /// return of the given closure `f`.
    pub fn with_mode<TMODE, R>(&mut self, f: impl FnOnce(&mut Pin<P, N, TMODE>) -> R) -> R
    where
        TMODE: MultiMode + Sealed,
        Pin<P, N, TMODE>: Sealed,
    {
        let mut temp_pin: Pin<P, N, TMODE> = Pin::new();
        TMODE::set_regs(self.port(), self.pin());
        let ret = f(&mut temp_pin);
        MODE::set_regs(self.port(), self.pin());
        ret
    }

    /// Convert this pin into an erased pin, where the Port and Pin are not stored as type states
    pub fn into_erased_pin(self) -> ErasedPin<MODE> {
        ErasedPin::new(self.port(), self.pin())
    }

    /// Convert this pin into a dynamic pin, with no type states
    pub fn into_dynamic_pin(self) -> DynamicPin {
        DynamicPin::new(self.port(), self.pin(), MODE::dynamic_mode())
    }
}

/// Port and Pin info
pub trait PinInfo {
    /// Port id for the port which contains this pin
    fn port(&self) -> PortId;

    /// Pin number
    fn pin(&self) -> u8;
}

impl<const P: char, const N: u8, MODE> PinInfo for Pin<P, N, MODE> {
    fn port(&self) -> PortId {
        // SAFETY: the `P` generic type parameter is guaranteed to denote a valid port id
        P.try_into().unwrap()
    }

    fn pin(&self) -> u8 {
        N
    }
}

/// `InputPin` implementation for trait from `embedded-hal`
impl<const P: char, const N: u8, MODE> InputPin for Pin<P, N, MODE>
where
    MODE: InputMode,
{
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if port::ports::din_dis(self.port()) {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(pins::din(self.port(), self.pin()))
        }
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if port::ports::din_dis(self.port()) {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(!pins::din(self.port(), self.pin()))
        }
    }
}

/// `OutputPin` implementation for trait from `embedded-hal`
impl<const P: char, const N: u8, MODE> OutputPin for Pin<P, N, MODE>
where
    MODE: OutputMode,
{
    fn set_low(&mut self) -> Result<(), Self::Error> {
        pins::set_dout(self.port(), self.pin(), false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        pins::set_dout(self.port(), self.pin(), true);
        Ok(())
    }
}

/// `StatefulOutputPin` implementation for trait from `embedded-hal`
impl<const P: char, const N: u8, SMODE> StatefulOutputPin for Pin<P, N, Out<SMODE>>
where
    Out<SMODE>: OutputMode,
{
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if port::ports::din_dis(self.port()) {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(pins::din(self.port(), self.pin()))
        }
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if port::ports::din_dis(self.port()) {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(!pins::din(self.port(), self.pin()))
        }
    }
}

/// `StatefulOutputPin` (`Alt` output mode) implementation for trait from `embedded-hal`
impl<const P: char, const N: u8, SMODE> StatefulOutputPin for Pin<P, N, OutAlt<SMODE>>
where
    OutAlt<SMODE>: OutputMode,
{
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if port::ports::din_dis_alt(self.port()) {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(pins::din(self.port(), self.pin()))
        }
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if port::ports::din_dis_alt(self.port()) {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(!pins::din(self.port(), self.pin()))
        }
    }
}

impl<const P: char, const N: u8, MODE> ErrorType for Pin<P, N, MODE> {
    type Error = GpioError;
}

impl<MODE> Sealed for Pin<'A', 0, MODE> {}
impl<MODE> Sealed for Pin<'A', 1, MODE> {}
impl<MODE> Sealed for Pin<'A', 2, MODE> {}
impl<MODE> Sealed for Pin<'A', 3, MODE> {}
impl<MODE> Sealed for Pin<'A', 4, MODE> {}
impl<MODE> Sealed for Pin<'A', 5, MODE> {}
impl<MODE> Sealed for Pin<'B', 11, MODE> {}
impl<MODE> Sealed for Pin<'B', 12, MODE> {}
impl<MODE> Sealed for Pin<'B', 13, MODE> {}
impl<MODE> Sealed for Pin<'B', 14, MODE> {}
impl<MODE> Sealed for Pin<'B', 15, MODE> {}
impl<MODE> Sealed for Pin<'C', 6, MODE> {}
impl<MODE> Sealed for Pin<'C', 7, MODE> {}
impl<MODE> Sealed for Pin<'C', 8, MODE> {}
impl<MODE> Sealed for Pin<'C', 9, MODE> {}
impl<MODE> Sealed for Pin<'C', 10, MODE> {}
impl<MODE> Sealed for Pin<'C', 11, MODE> {}
impl<MODE> Sealed for Pin<'D', 9, MODE> {}
impl<MODE> Sealed for Pin<'D', 10, MODE> {}
impl<MODE> Sealed for Pin<'D', 11, MODE> {}
impl<MODE> Sealed for Pin<'D', 12, MODE> {}
impl<MODE> Sealed for Pin<'D', 13, MODE> {}
impl<MODE> Sealed for Pin<'D', 14, MODE> {}
impl<MODE> Sealed for Pin<'D', 15, MODE> {}
impl<MODE> Sealed for Pin<'F', 0, MODE> {}
impl<MODE> Sealed for Pin<'F', 1, MODE> {}
impl<MODE> Sealed for Pin<'F', 2, MODE> {}
impl<MODE> Sealed for Pin<'F', 3, MODE> {}
impl<MODE> Sealed for Pin<'F', 4, MODE> {}
impl<MODE> Sealed for Pin<'F', 5, MODE> {}
impl<MODE> Sealed for Pin<'F', 6, MODE> {}
impl<MODE> Sealed for Pin<'F', 7, MODE> {}

/// Pin mode types (type state)
pub(crate) mod mode {
    use crate::gpio::dynamic::DynamicMode;
    use crate::gpio::pin::pins;
    use crate::gpio::port::PortId;
    use crate::pac::gpio::port_a::model::MODE0;
    use crate::Sealed;
    use core::marker::PhantomData;

    /// Disabled mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Disabled;
    /// Disabled with pull-up mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct DisabledPu;
    /// Input floating mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InFloat;
    /// Input with filter mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InFilt;
    /// Input with pull-up mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InPu;
    /// Input with pull-up and filter mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InPuFilt;
    /// Input with pull-down mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InPd;
    /// Input with pull-down and filter mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InPdFilt;
    /// Output open source mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OutOs;
    /// Output open source, pull-down mode (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OutOsPd;

    /// Output push-pull mode (type state)
    pub type OutPp = Out<PushPull>;
    /// Output open drain mode (type state)
    pub type OutOd = Out<OpenDrain>;
    /// Output open drain with filter mode (type state)
    pub type OutOdFilt = Out<OpenDrainFilter>;
    /// Output open drain pull-up mode (type state)
    pub type OutOdPu = Out<OpenDrainPullUp>;
    /// Output open drain pull-up with filter mode (type state)
    pub type OutOdPuFilt = Out<OpenDrainPullUpFilter>;
    /// Alternate Output push-pull mode (type state)
    pub type OutPpAlt = OutAlt<PushPull>;
    /// Alternate Output open drain mode (type state)
    pub type OutOdAlt = OutAlt<OpenDrain>;
    /// Alternate Output open drain with filter mode (type state)
    pub type OutOdFiltAlt = OutAlt<OpenDrainFilter>;
    /// Alternate Output open drain pull-up mode (type state)
    pub type OutOdPuAlt = OutAlt<OpenDrainPullUp>;
    /// Alternate Output open drain pull-up with filter mode (type state)
    pub type OutOdPuFiltAlt = OutAlt<OpenDrainPullUpFilter>;

    /// The mode of an Output pin which uses the Primary configuration (as opposed to Alternate) (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Out<SMODE> {
        _smode: PhantomData<SMODE>,
    }
    /// The mode of an Output pin which uses the Alternate configuration (as opposed to Primary) (type state)
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OutAlt<SMODE> {
        _smode: PhantomData<SMODE>,
    }
    /// Sub-mode for an output pin mode, either [`Out`], or [`OutAlt`]
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct PushPull;
    /// Sub-mode for an output pin mode, either [`Out`], or [`OutAlt`]
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OpenDrain;
    /// Sub-mode for an output pin mode, either [`Out`], or [`OutAlt`]
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OpenDrainFilter;
    /// Sub-mode for an output pin mode, either [`Out`], or [`OutAlt`]
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OpenDrainPullUp;
    /// Sub-mode for an output pin mode, either [`Out`], or [`OutAlt`]
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct OpenDrainPullUpFilter;

    /// Analog pin mode (type state)
    ///
    /// All pins which implement `MultiMode` can also be converted to `Analog` mode
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Analog;

    impl Sealed for Disabled {}
    impl Sealed for DisabledPu {}
    impl Sealed for Analog {}
    impl Sealed for InFloat {}
    impl Sealed for InFilt {}
    impl Sealed for InPu {}
    impl Sealed for InPuFilt {}
    impl Sealed for InPd {}
    impl Sealed for InPdFilt {}
    impl Sealed for OutPp {}
    impl Sealed for OutOs {}
    impl Sealed for OutOsPd {}
    impl Sealed for OutOd {}
    impl Sealed for OutOdFilt {}
    impl Sealed for OutOdPu {}
    impl Sealed for OutOdPuFilt {}
    impl Sealed for OutPpAlt {}
    impl Sealed for OutOdAlt {}
    impl Sealed for OutOdFiltAlt {}
    impl Sealed for OutOdPuAlt {}
    impl Sealed for OutOdPuFiltAlt {}

    /// Marker trait for Input mode pins
    pub trait InputMode: Sealed {}
    impl InputMode for InFloat {}
    impl InputMode for InFilt {}
    impl InputMode for InPu {}
    impl InputMode for InPuFilt {}
    impl InputMode for InPd {}
    impl InputMode for InPdFilt {}

    /// Marker trait for Output mode pins
    pub trait OutputMode: Sealed {}
    impl OutputMode for OutPp {}
    impl OutputMode for OutOs {}
    impl OutputMode for OutOsPd {}
    impl OutputMode for OutOd {}
    impl OutputMode for OutOdFilt {}
    impl OutputMode for OutOdPu {}
    impl OutputMode for OutOdPuFilt {}
    impl OutputMode for OutPpAlt {}
    impl OutputMode for OutOdAlt {}
    impl OutputMode for OutOdFiltAlt {}
    impl OutputMode for OutOdPuAlt {}
    impl OutputMode for OutOdPuFiltAlt {}

    /// Trait for transitioning a pin from one mode to another, where the possible target modes are:
    ///
    /// [`Disabled`], [`DisabledPu`],
    /// [`InFloat`], [`InFilt`], [`InPu`], [`InPuFilt`], [`InPd`], [`InPdFilt`],
    /// [`OutPp`], [`OutOs`], [`OutOsPd`], [`OutOd`], [`OutOdFilt`], [`OutOdPu`], [`OutOdPuFilt`],
    /// [`OutPpAlt`], [`OutOdAlt`], [`OutOdFiltAlt`], [`OutOdPuAlt`], [`OutOdPuFiltAlt`], [`Analog`]
    pub trait MultiMode: Sealed {
        /// Set the peripheral registers such that they match the `MODE` of the `pin` in `port`
        fn set_regs(port: PortId, pin: u8);

        /// Get the `DynamicMode` variant corresponding to the mode type which implements this trait
        fn dynamic_mode() -> DynamicMode;
    }

    impl MultiMode for Disabled {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Disabled);
            pins::set_dout(port, pin, false);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::Disabled
        }
    }

    impl MultiMode for DisabledPu {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Disabled);
            pins::set_dout(port, pin, true);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::DisabledPu
        }
    }

    impl MultiMode for Analog {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Disabled);
            pins::set_dout(port, pin, false);
            pins::set_ovt(port, pin, false);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::Analog
        }
    }

    impl MultiMode for InFloat {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Input);
            pins::set_dout(port, pin, false);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::InFloat
        }
    }

    impl MultiMode for InFilt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Input);
            pins::set_dout(port, pin, true);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::InFilt
        }
    }

    impl MultiMode for InPu {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Inputpull);
            pins::set_dout(port, pin, true);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::InPu
        }
    }

    impl MultiMode for InPuFilt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Inputpullfilter);
            pins::set_dout(port, pin, true);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::InPuFilt
        }
    }

    impl MultiMode for InPd {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Inputpull);
            pins::set_dout(port, pin, false);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::InPd
        }
    }

    impl MultiMode for InPdFilt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Inputpullfilter);
            pins::set_dout(port, pin, false);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::InPdFilt
        }
    }

    impl MultiMode for OutPp {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Pushpull);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutPp
        }
    }

    impl MultiMode for OutOs {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredor);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOs
        }
    }

    impl MultiMode for OutOsPd {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredorpulldown);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOsPd
        }
    }

    impl MultiMode for OutOd {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredand);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOd
        }
    }

    impl MultiMode for OutOdFilt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandfilter);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdFilt
        }
    }

    impl MultiMode for OutOdPu {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandpullup);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdPu
        }
    }

    impl MultiMode for OutOdPuFilt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandpullupfilter);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdPuFilt
        }
    }

    impl MultiMode for OutPpAlt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Pushpullalt);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutPpAlt
        }
    }

    impl MultiMode for OutOdAlt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandalt);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdAlt
        }
    }

    impl MultiMode for OutOdFiltAlt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandaltfilter);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdFiltAlt
        }
    }

    impl MultiMode for OutOdPuAlt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandaltpullup);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdPuAlt
        }
    }

    impl MultiMode for OutOdPuFiltAlt {
        #[inline(always)]
        fn set_regs(port: PortId, pin: u8) {
            pins::mode_set(port, pin, MODE0::Wiredandaltpullupfilter);
            pins::set_ovt(port, pin, true);
        }

        fn dynamic_mode() -> DynamicMode {
            DynamicMode::OutOdPuFiltAlt
        }
    }
}

/// Configure GPIO peripheral registers values for individual pins
pub(crate) mod pins {
    use efm32pg1b_pac::gpio::port_a::model::MODE0;

    use crate::gpio::port::{ports, PortId};

    /// Set the Mode for a given pin `N` in port `P`
    #[inline(always)]
    pub(crate) fn mode_set(port: PortId, pin: u8, iomode: MODE0) {
        match pin {
            0..=7 => {
                ports::get(port).model().modify(|_, w| {
                    match pin {
                        0 => w.mode0(),
                        1 => w.mode1(),
                        2 => w.mode2(),
                        3 => w.mode3(),
                        4 => w.mode4(),
                        5 => w.mode5(),
                        6 => w.mode6(),
                        7 => w.mode7(),
                        _ => unreachable!(),
                    }
                    .variant(iomode)
                });
            }
            8..=15 => {
                ports::get(port).modeh().modify(|_, w| {
                    match pin {
                        8 => w.mode8(),
                        9 => w.mode9(),
                        10 => w.mode10(),
                        11 => w.mode11(),
                        12 => w.mode12(),
                        13 => w.mode13(),
                        14 => w.mode14(),
                        15 => w.mode15(),
                        _ => unreachable!(),
                    }
                    .variant(iomode)
                });
            }
            _ => unreachable!(),
        }
    }

    /// Get the Data Out for a given `pin` in `port`
    #[inline(always)]
    pub(crate) fn dout(port: PortId, pin: u8) -> bool {
        (ports::get(port).dout().read().pins_dout().bits() & (1u16 << pin)) != 0
    }

    /// Set the Data Out for a given `pin` in `port`
    #[inline(always)]
    pub(crate) fn set_dout(port: PortId, pin: u8, dout: bool) {
        ports::get(port).dout().modify(|r, w| match dout {
            true => unsafe { w.pins_dout().bits(r.bits() as u16 | (1u16 << pin)) },
            false => unsafe { w.pins_dout().bits(r.bits() as u16 & !(1u16 << pin)) },
        });
    }

    /// Get the Data In for a given pin `pin` in `port`
    #[inline(always)]
    pub(crate) fn din(port: PortId, pin: u8) -> bool {
        ports::get(port).din().read().pins_din().bits() as u16 & (1u16 << pin) != 0
    }

    /// Return `true` if Over Voltage Tolerance is enabled for a given `pin` in `port`
    ///
    /// OVT is enabled by default for all pins
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn ovt(port: PortId, pin: u8) -> bool {
        ports::get(port).ovt_dis().read().pins_ovt_dis().bits() & (1u16 << pin) == 0
    }

    /// Set the Over Voltage Tolerance for a given `pin` in `port`
    ///
    /// OVT is enabled by default for all pins
    #[inline(always)]
    pub(crate) fn set_ovt(port: PortId, pin: u8, enabled: bool) {
        // The `GPIO_Px_OVTDIS` register uses raised flags for each pin to signal that OVT is _disabled_
        ports::get(port).ovt_dis().modify(|r, w| match enabled {
            true => unsafe {
                w.pins_ovt_dis()
                    .bits(r.pins_ovt_dis().bits() & !(1u16 << pin))
            },
            false => unsafe {
                w.pins_ovt_dis()
                    .bits(r.pins_ovt_dis().bits() | (1u16 << pin))
            },
        });
    }
}

/// Implement `fmt::Debug` and `defmt::Format` for [`Pin`] types with given `mode`
///
/// Takes as parameters the Pin Mode type, and a str representation of the Pin Mode type name
macro_rules! impl_fmt_debug {
    ($mode:ty, $mode_name: literal) => {
        impl<const P: char, const N: u8> fmt::Debug for Pin<P, N, $mode> {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_fmt(format_args!("Pin<'{}',{},{}>", P, N, $mode_name))
            }
        }

        #[cfg(feature = "defmt")]
        impl<const P: char, const N: u8> defmt::Format for Pin<P, N, $mode> {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "Pin<'{}',{},{}>", P, N, $mode_name);
            }
        }
    };
}

pub(crate) use impl_fmt_debug;

impl_fmt_debug!(mode::Disabled, "Disabled");
impl_fmt_debug!(mode::DisabledPu, "DisabledPu");
impl_fmt_debug!(mode::InFloat, "InFloat");
impl_fmt_debug!(mode::InFilt, "InFilt");
impl_fmt_debug!(mode::InPu, "InPu");
impl_fmt_debug!(mode::InPuFilt, "InPuFilt");
impl_fmt_debug!(mode::InPd, "InPd");
impl_fmt_debug!(mode::InPdFilt, "InPdFilt");
impl_fmt_debug!(mode::OutOs, "OutOs");
impl_fmt_debug!(mode::OutOsPd, "OutOsPd");
impl_fmt_debug!(mode::Out<mode::PushPull>, "OutPp");
impl_fmt_debug!(mode::Out<mode::OpenDrain>, "OutOd");
impl_fmt_debug!(mode::Out<mode::OpenDrainFilter>, "OutOdFilt");
impl_fmt_debug!(mode::Out<mode::OpenDrainPullUp>, "OutOdPu");
impl_fmt_debug!(mode::Out<mode::OpenDrainPullUpFilter>, "OutOdPuFilt");
impl_fmt_debug!(mode::OutAlt<mode::PushPull>, "OutPpAlt");
impl_fmt_debug!(mode::OutAlt<mode::OpenDrain>, "OutOdAlt");
impl_fmt_debug!(mode::OutAlt<mode::OpenDrainFilter>, "OutOdFiltAlt");
impl_fmt_debug!(mode::OutAlt<mode::OpenDrainPullUp>, "OutOdPuAlt");
impl_fmt_debug!(mode::OutAlt<mode::OpenDrainPullUpFilter>, "OutOdPuFiltAlt");
impl_fmt_debug!(mode::Analog, "Analog");
