//! Erased pins
//!
//! This module implements the recommendations of [C-ERASED-PIN](https://docs.rust-embedded.org/book/design-patterns/hal/gpio.html#pin-types-provide-methods-to-erase-pin-and-port-c-erased-pin):
//!
//! Pins should provide type erasure methods that move their properties from compile time to runtime, and allow more
//! flexibility in applications.
//!
//! ```rust,no_run
//! let p = pac::Peripherals::take().unwrap();
//! let mut gpio = Gpio::new(p.gpio);
//!
//! let pb11 = gpio.pb11.into_erased_pin().into_mode::<InPu>();
//! let pb12 = gpio.pb12.into_erased_pin().into_mode::<InPu>();
//! let pb13 = gpio.pb13.into_erased_pin().into_mode::<InPu>();
//! let pb14 = gpio.pb14.into_erased_pin().into_mode::<InPu>();
//!
//! // Erased pins with the same mode can be aggregated
//! let pin_array = [pb11, pb12, pb13, pb14];
//! ```

use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

use crate::{
    gpio::{
        pin::{
            mode::{self, InputMode, MultiMode, Out, OutAlt, OutputMode},
            pins, PinInfo,
        },
        port::{self, PortId},
        GpioError,
    },
    Sealed,
};
use core::{fmt, marker::PhantomData};

/// Erased Pin
///
/// [C-ERASED-PIN](https://docs.rust-embedded.org/book/design-patterns/hal/gpio.html#pin-types-provide-methods-to-erase-pin-and-port-c-erased-pin)
pub struct ErasedPin<MODE> {
    /// Most significant nibble is the port id, least significant nibble is the pin id
    port_pin: u8,
    _mode: PhantomData<MODE>,
}

impl<MODE> ErasedPin<MODE>
where
    MODE: MultiMode + Sealed,
    ErasedPin<MODE>: Sealed,
{
    pub(crate) fn new(port: PortId, pin: u8) -> Self {
        Self {
            port_pin: ((port as u8) << 4) | pin,
            _mode: PhantomData,
        }
    }

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
    pub fn into_mode<NMODE>(self) -> ErasedPin<NMODE>
    where
        NMODE: MultiMode + Sealed,
        ErasedPin<NMODE>: Sealed,
    {
        NMODE::set_regs(self.port(), self.pin());
        ErasedPin::new(self.port(), self.pin())
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
    pub fn with_mode<TMODE, R>(&mut self, f: impl FnOnce(&mut ErasedPin<TMODE>) -> R) -> R
    where
        TMODE: MultiMode + Sealed,
        ErasedPin<TMODE>: Sealed,
    {
        let mut temp_pin = ErasedPin::new(self.port(), self.pin());
        TMODE::set_regs(self.port(), self.pin());
        let ret = f(&mut temp_pin);
        MODE::set_regs(self.port(), self.pin());
        ret
    }
}

impl<MODE> PinInfo for ErasedPin<MODE> {
    fn port(&self) -> PortId {
        // SAFETY: the `pin_port` value was composed with a valid `PortId` value, so the reverse operation cannot fail
        (self.port_pin >> 4 & 0x0F).try_into().unwrap()
    }

    fn pin(&self) -> u8 {
        self.port_pin & 0x0F
    }
}

impl<MODE> Sealed for ErasedPin<MODE> {}

/// `InputPin` implementation for trait from `embedded-hal`
impl<MODE> InputPin for ErasedPin<MODE>
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
impl<MODE> OutputPin for ErasedPin<MODE>
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
impl<SMODE> StatefulOutputPin for ErasedPin<Out<SMODE>>
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
impl<SMODE> StatefulOutputPin for ErasedPin<OutAlt<SMODE>>
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

impl<MODE> ErrorType for ErasedPin<MODE> {
    type Error = GpioError;
}

macro_rules! impl_fmt_debug_erased_pin {
    ($mode:ty, $mode_name: literal) => {
        impl fmt::Debug for ErasedPin<$mode> {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_fmt(format_args!(
                    "ErasedPin({}{})<{}>",
                    core::convert::Into::<char>::into(self.port()),
                    self.pin(),
                    $mode_name
                ))
            }
        }

        #[cfg(feature = "defmt")]
        impl defmt::Format for ErasedPin<$mode> {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ErasedPin({}{})<{}>",
                    core::convert::Into::<char>::into(self.port()),
                    self.pin(),
                    $mode_name
                );
            }
        }
    };
}

impl_fmt_debug_erased_pin!(mode::Disabled, "Disabled");
impl_fmt_debug_erased_pin!(mode::DisabledPu, "DisabledPu");
impl_fmt_debug_erased_pin!(mode::InFloat, "InFloat");
impl_fmt_debug_erased_pin!(mode::InFilt, "InFilt");
impl_fmt_debug_erased_pin!(mode::InPu, "InPu");
impl_fmt_debug_erased_pin!(mode::InPuFilt, "InPuFilt");
impl_fmt_debug_erased_pin!(mode::InPd, "InPd");
impl_fmt_debug_erased_pin!(mode::InPdFilt, "InPdFilt");
impl_fmt_debug_erased_pin!(mode::OutOs, "OutOs");
impl_fmt_debug_erased_pin!(mode::OutOsPd, "OutOsPd");
impl_fmt_debug_erased_pin!(mode::Out<mode::PushPull>, "OutPp");
impl_fmt_debug_erased_pin!(mode::Out<mode::OpenDrain>, "OutOd");
impl_fmt_debug_erased_pin!(mode::Out<mode::OpenDrainFilter>, "OutOdFilt");
impl_fmt_debug_erased_pin!(mode::Out<mode::OpenDrainPullUp>, "OutOdPu");
impl_fmt_debug_erased_pin!(mode::Out<mode::OpenDrainPullUpFilter>, "OutOdPuFilt");
impl_fmt_debug_erased_pin!(mode::OutAlt<mode::PushPull>, "OutPpAlt");
impl_fmt_debug_erased_pin!(mode::OutAlt<mode::OpenDrain>, "OutOdAlt");
impl_fmt_debug_erased_pin!(mode::OutAlt<mode::OpenDrainFilter>, "OutOdFiltAlt");
impl_fmt_debug_erased_pin!(mode::OutAlt<mode::OpenDrainPullUp>, "OutOdPuAlt");
impl_fmt_debug_erased_pin!(mode::OutAlt<mode::OpenDrainPullUpFilter>, "OutOdPuFiltAlt");
impl_fmt_debug_erased_pin!(mode::Analog, "Analog");
