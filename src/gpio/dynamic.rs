//! Dynamic pins
//!

use crate::{
    gpio::{
        pin::{self, mode::MultiMode, pins, PinInfo},
        port::{self, PortId},
        GpioError,
    },
    Sealed,
};
use core::fmt;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

pub struct DynamicPin {
    /// Most significant nibble is the port id, least significant nibble is the pin id
    port_pin: u8,
    /// Pin mode
    mode: DynamicMode,
}

impl DynamicPin {
    pub(crate) fn new(port: PortId, pin: u8, mode: DynamicMode) -> Self {
        Self {
            port_pin: ((port as u8) << 4) | pin,
            mode,
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
    pub fn into_mode<MODE>(self) -> Self
    where
        MODE: MultiMode + Sealed,
    {
        MODE::set_regs(self.port(), self.pin());
        Self::new(self.port(), self.pin(), MODE::dynamic_mode())
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
    pub fn with_mode<TMODE, R>(&mut self, f: impl FnOnce(&mut DynamicPin) -> R) -> R
    where
        TMODE: MultiMode + Sealed,
    {
        let mut temp_pin = DynamicPin::new(self.port(), self.pin(), TMODE::dynamic_mode());
        TMODE::set_regs(self.port(), self.pin());
        let ret = f(&mut temp_pin);
        self.mode.set_regs(self.port(), self.pin());
        ret
    }
}

/// `InputPin` implementation for trait from `embedded-hal`
/// Allows treating Outout pins as Input pins, since the uC allows it
/// TODO: restrict this to input pins?
impl InputPin for DynamicPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        if !self.mode.readable() {
            Err(GpioError::InvalidMode(self.mode))
        } else if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if ((self.mode.readable_input() || self.mode.readable_out())
            && port::ports::din_dis(self.port()))
            || (self.mode.readable_out_alt() && port::ports::din_dis_alt(self.port()))
        {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(pins::din(self.port(), self.pin()))
        }
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_high()?)
    }
}

/// `OutputPin` implementation for trait from `embedded-hal`
impl OutputPin for DynamicPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        if !self.mode.writable() {
            Err(GpioError::InvalidMode(self.mode))
        } else if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else {
            pins::set_dout(self.port(), self.pin(), false);
            Ok(())
        }
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        if !self.mode.writable() {
            Err(GpioError::InvalidMode(self.mode))
        } else if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else {
            pins::set_dout(self.port(), self.pin(), true);
            Ok(())
        }
    }
}

/// `StatefulOutputPin` implementation for trait from `embedded-hal`
impl StatefulOutputPin for DynamicPin {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        if !crate::gpio::is_enabled() {
            Err(GpioError::GpioDisabled)
        } else if !self.mode.writable() {
            Err(GpioError::InvalidMode(self.mode))
        } else {
            // Return the current state of the _output_, not of the input register
            Ok(pins::dout(self.port(), self.pin()))
        }
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_set_high()?)
    }
}

impl ErrorType for DynamicPin {
    type Error = GpioError;
}

/// Dynamic pin modes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum DynamicMode {
    /// Disabled
    Disabled,
    /// Disabled with pull-up
    DisabledPu,
    /// Analog pin which can be used with the ADC/DAC peripherals
    Analog,
    /// Input floating
    InFloat,
    /// Input with filter
    InFilt,
    /// Input with pull-up
    InPu,
    /// Input with pull-up and filter
    InPuFilt,
    /// Input with pull-down
    InPd,
    /// Input with pull-down and filter
    InPdFilt,
    /// Output push-pull
    OutPp,
    /// Output open source
    OutOs,
    /// Output open source, pull-down
    OutOsPd,
    /// Output open drain
    OutOd,
    /// Output open drain with filter
    OutOdFilt,
    /// Output open drain pull-up
    OutOdPu,
    /// Output open drain pull-up with filter
    OutOdPuFilt,
    /// Alternate Output push-pull
    OutPpAlt,
    /// Alternate Output open drain
    OutOdAlt,
    /// Alternate Output open drain with filter
    OutOdFiltAlt,
    /// Alternate Output open drain pull-up
    OutOdPuAlt,
    /// Alternate Output open drain pull-up with filter
    OutOdPuFiltAlt,
}

impl DynamicMode {
    fn set_regs(&self, port: PortId, pin: u8) {
        match self {
            DynamicMode::Disabled => pin::mode::Disabled::set_regs(port, pin),
            DynamicMode::DisabledPu => pin::mode::DisabledPu::set_regs(port, pin),
            DynamicMode::Analog => pin::mode::Analog::set_regs(port, pin),
            DynamicMode::InFloat => pin::mode::InFloat::set_regs(port, pin),
            DynamicMode::InFilt => pin::mode::InFilt::set_regs(port, pin),
            DynamicMode::InPu => pin::mode::InPu::set_regs(port, pin),
            DynamicMode::InPuFilt => pin::mode::InPuFilt::set_regs(port, pin),
            DynamicMode::InPd => pin::mode::InPd::set_regs(port, pin),
            DynamicMode::InPdFilt => pin::mode::InPdFilt::set_regs(port, pin),
            DynamicMode::OutPp => pin::mode::OutPp::set_regs(port, pin),
            DynamicMode::OutOs => pin::mode::OutOs::set_regs(port, pin),
            DynamicMode::OutOsPd => pin::mode::OutOsPd::set_regs(port, pin),
            DynamicMode::OutOd => pin::mode::OutOd::set_regs(port, pin),
            DynamicMode::OutOdFilt => pin::mode::OutOdFilt::set_regs(port, pin),
            DynamicMode::OutOdPu => pin::mode::OutOdPu::set_regs(port, pin),
            DynamicMode::OutOdPuFilt => pin::mode::OutOdPuFilt::set_regs(port, pin),
            DynamicMode::OutPpAlt => pin::mode::OutPpAlt::set_regs(port, pin),
            DynamicMode::OutOdAlt => pin::mode::OutOdAlt::set_regs(port, pin),
            DynamicMode::OutOdFiltAlt => pin::mode::OutOdFiltAlt::set_regs(port, pin),
            DynamicMode::OutOdPuAlt => pin::mode::OutOdPuAlt::set_regs(port, pin),
            DynamicMode::OutOdPuFiltAlt => pin::mode::OutOdPuFiltAlt::set_regs(port, pin),
        }
    }

    fn readable(&self) -> bool {
        !matches!(
            self,
            DynamicMode::Disabled | DynamicMode::DisabledPu | DynamicMode::Analog
        )
    }

    fn readable_input(&self) -> bool {
        matches!(
            self,
            DynamicMode::InFloat
                | DynamicMode::InFilt
                | DynamicMode::InPu
                | DynamicMode::InPuFilt
                | DynamicMode::InPd
                | DynamicMode::InPdFilt
        )
    }

    fn readable_out(&self) -> bool {
        matches!(
            self,
            DynamicMode::OutPp
                | DynamicMode::OutOs
                | DynamicMode::OutOsPd
                | DynamicMode::OutOd
                | DynamicMode::OutOdFilt
                | DynamicMode::OutOdPu
                | DynamicMode::OutOdPuFilt
        )
    }

    fn readable_out_alt(&self) -> bool {
        matches!(
            self,
            DynamicMode::OutPpAlt
                | DynamicMode::OutOdAlt
                | DynamicMode::OutOdFiltAlt
                | DynamicMode::OutOdPuAlt
                | DynamicMode::OutOdPuFiltAlt
        )
    }

    fn writable(&self) -> bool {
        !matches!(
            self,
            DynamicMode::Disabled
                | DynamicMode::DisabledPu
                | DynamicMode::Analog
                | DynamicMode::InFloat
                | DynamicMode::InFilt
                | DynamicMode::InPu
                | DynamicMode::InPuFilt
                | DynamicMode::InPd
                | DynamicMode::InPdFilt
        )
    }

    const fn name(&self) -> &'static str {
        match self {
            DynamicMode::Disabled => "Disabled",
            DynamicMode::DisabledPu => "DisabledPu",
            DynamicMode::Analog => "Analog",
            DynamicMode::InFloat => "InFloat",
            DynamicMode::InFilt => "InFilt",
            DynamicMode::InPu => "InPu",
            DynamicMode::InPuFilt => "InPuFilt",
            DynamicMode::InPd => "InPd",
            DynamicMode::InPdFilt => "InPdFilt",
            DynamicMode::OutPp => "OutPp",
            DynamicMode::OutOs => "OutOs",
            DynamicMode::OutOsPd => "OutOsPd",
            DynamicMode::OutOd => "OutOd",
            DynamicMode::OutOdFilt => "OutOdFilt",
            DynamicMode::OutOdPu => "OutOdPu",
            DynamicMode::OutOdPuFilt => "OutOdPuFilt",
            DynamicMode::OutPpAlt => "OutPpAlt",
            DynamicMode::OutOdAlt => "OutOdAlt",
            DynamicMode::OutOdFiltAlt => "OutOdFiltAlt",
            DynamicMode::OutOdPuAlt => "OutOdPuAlt",
            DynamicMode::OutOdPuFiltAlt => "OutOdPuFiltAlt",
        }
    }
}

impl PinInfo for DynamicPin {
    fn port(&self) -> PortId {
        // SAFETY: the `pin_port` value was composed with a valid `PortId` value because a `DynamicPin` can only be
        // converted from a zero-size sype `Pin`, so the reverse operation cannot fail
        (self.port_pin >> 4 & 0x0F).try_into().unwrap()
    }

    fn pin(&self) -> u8 {
        self.port_pin & 0x0F
    }
}

impl Sealed for DynamicPin {}

impl fmt::Debug for DynamicPin {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "DynamicPin{{{}{},{}}}",
            core::convert::Into::<char>::into(self.port()),
            self.pin(),
            self.mode.name()
        ))
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for DynamicPin {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "DynamicPin{{{}{},{}}}",
            core::convert::Into::<char>::into(self.port()),
            self.pin(),
            self.mode.name()
        );
    }
}
