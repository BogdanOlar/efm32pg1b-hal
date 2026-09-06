//! Debug pins
//!
//! This module allows the HAL to use the Debug pins as normal GPIO pins, provided that the `use_debug_pins` feature
//! flag is enabled.
//!
//! ```rust,no_run
//! let p = ::take().unwrap();
//! let mut gpio = GPIO::new(p.gpio);
//!
//! // Be aware that converting the Debug pins into GPIO pins may fail
//! // (e.g. if the debugger is attached), so calling `unwrap()`
//! // may have unintended consequences
//! let pins = gpio.debug_pins.into_gpio_pins().unwrap();
//!
//! // the resulted struct contains the four debug pins, in disabled mode
//! let pf0 = pins.pf0;
//! let pf1 = pins.pf1;
//! let pf2 = pins.pf2;
//! let pf3 = pins.pf3;
//! ```

use crate::gpio::{
    pin::Pin,
    port::{ports, DataInCtrl, PortId},
    Disabled, GpioError,
};
use core::fmt;

/// Debug pin mode
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DbgPin;

/// All Debug (SWD/JTAG) pins (currently enabled)
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DebugPinsEnabled {
    _clk: Pin<'F', 0, DbgPin>,
    _io: Pin<'F', 1, DbgPin>,
    _tdi: Pin<'F', 2, DbgPin>,
    _tdo: Pin<'F', 3, DbgPin>,
}

impl DebugPinsEnabled {
    pub(crate) fn new() -> Self {
        Self {
            _clk: Pin::new(),
            _io: Pin::new(),
            _tdi: Pin::new(),
            _tdo: Pin::new(),
        }
    }

    /// Enable debug pins, make sure that Data In Disabled is clear for port `F`
    ///
    /// Takes pins `pf0`,`pf1`,`pf2` and `pf3` (in `Disabled` mode) as parameters
    pub fn from_pins(
        _clk: Pin<'F', 0, Disabled>,
        _io: Pin<'F', 1, Disabled>,
        _tdi: Pin<'F', 2, Disabled>,
        _tdo: Pin<'F', 3, Disabled>,
    ) -> Self {
        let gpio = crate::pac::GPIO;

        // Make sure Data In Disable is clear for port `F`
        ports::set_din_dis(PortId::F, DataInCtrl::Enabled);

        // By default, the debug pins are enabled, so we can just reset the GPIO_ROUTEPEN register to its default value
        gpio.routepen().write_value(Default::default());

        Self::new()
    }

    /// Try to convert the debug pins into gpio pins
    pub fn into_gpio_pins(self) -> Result<DebugPinsDisabled, GpioError> {
        self.try_into()
    }
}

/// All gpio pins which can be used as Debug (SWD/JTAG) pins
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DebugPinsDisabled {
    /// Swd Clk pin
    pub pf0: Pin<'F', 0, Disabled>,
    /// Swd IO pin
    pub pf1: Pin<'F', 1, Disabled>,
    /// TDI pin
    pub pf2: Pin<'F', 2, Disabled>,
    /// TDO pin
    pub pf3: Pin<'F', 3, Disabled>,
}

impl DebugPinsDisabled {
    fn new() -> Self {
        // make sure to call the `into_mode` method which actually sets the registers according to the pin mode
        Self {
            pf0: Pin::<'F', 0, Disabled>::new().into_mode::<Disabled>(),
            pf1: Pin::<'F', 1, Disabled>::new().into_mode::<Disabled>(),
            pf2: Pin::<'F', 2, Disabled>::new().into_mode::<Disabled>(),
            pf3: Pin::<'F', 3, Disabled>::new().into_mode::<Disabled>(),
        }
    }
}

impl TryFrom<DebugPinsEnabled> for DebugPinsDisabled {
    type Error = GpioError;

    fn try_from(_pins: DebugPinsEnabled) -> Result<Self, Self::Error> {
        let gpio = crate::pac::GPIO;

        // Try to disable debug pins function
        gpio.routepen().write(|w| {
            w.set_swclktckpen(false);
            w.set_swdiotmspen(false);
            w.set_swvpen(false);
            w.set_tdipen(false);
            w.set_tdopen(false)
        });

        // If the debugger is still attached, then the write above will have had no effect
        match debug_pins_enabled() {
            true => Err(GpioError::DebugPinsEnabled),
            false => Ok(DebugPinsDisabled::new()),
        }
    }
}

/// Check if debug pins are enabled (pf0, pf1, pf2, pf3)
pub fn debug_pins_enabled() -> bool {
    let gpio = crate::pac::GPIO;

    gpio.routepen().read().swclktckpen() == true
        || gpio.routepen().read().swdiotmspen() == true
        || gpio.routepen().read().swvpen() == true
        || gpio.routepen().read().tdipen() == true
        || gpio.routepen().read().tdopen() == true
}

crate::gpio::pin::impl_fmt_debug!(DbgPin, "DbgPin");
