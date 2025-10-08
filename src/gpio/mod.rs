//! General Purpose Input / Output
//!
//! # Initialize HAL Gpio
//!
//! ```rust,no_run
//! // Acquire the PAC peripherals
//! let p = pac::Peripherals::take().unwrap();
//!
//! // Initialize HAL Gpio
//! let mut gpio = Gpio::new(p.gpio);
//! ```
//!
//! The initialized Gpio instance contains all available pins for the selected HW package.
//! Additional pins may be enabled with the `qfn32` or `qfn48` feature flags, depending on your controller's form factor
//!
//! All pins are in the [`Disabled`] mode by default, with the exception of the `Gpio::debug_pins` which can only be
//! modified if the `use_debug_pins` feature flag is enabled (see [`debug`] module for more info and examples).
//!
//! ```rust,no_run
//! // Zero-sized type pins
//! let mut led0 = gpio.pf4.into_mode::<OutPp>();
//! let mut btn0 = gpio.pf6.into_mode::<InFloat>();
//!
//! // Erased pins, where the port and pin info is stored in the
//! let mut led1 = gpio.pf5.into_erased_pin().into_mode::<OutPpAlt>();
//! let mut btn1 = gpio.pf7.into_erased_pin().into_mode::<InFilt>();
//! ```
//!
//! Ports are configured individually
//!
//! ```rust,no_run
//! gpio.port_f.set_drive_strength(DriveStrength::Strong);
//! ```
//!
//! # Use HAL Gpio
//!
//! [embedded-hal](https://github.com/rust-embedded/embedded-hal) version `1.0` defines traits with failible pin
//! operations. The return type is explicitly specified in the examples below just to help the understanding.
//!
//! ```rust,no_run
//! let button_read_result: Result<bool, GpioError> = btn0.is_high();
//! let led_result: Result<(), GpioError> = led0.set_high();
//! let led_result: Result<(), GpioError> = led0.toggle();
//! ```
//! Pins can be temporarily configured to a different mode while executing a given closure
//!
//! ```rust,no_run
//! let read_result: Result<bool, GpioError> = led0.with_mode::<InPuFilt, _>(|input_pin| input_pin.is_high());
//! ```
//!
//! # Modes
//!
//! Each GPIO pin can be configured with [`Pin::into_mode`] method, where the possible pin modes are listed below.
//! The same modes are also available when using the [`Pin::with_mode`] method of each Gpio pin.
//!
//! | Mode                      | Description                                                 |
//! |---------------------------|-------------------------------------------------------------|
//! |                           |                                                             |
//! | **[`Disabled`]**          | Disabled                                                    |
//! | **[`DisabledPu`]**        | Disabled with pull-up                                       |
//! | **[`Analog`]**            | Analog pin which can be used with the ADC/DAC peripherals   |
//! |                           |                                                             |
//! | **[`InFloat`]**           | Input floating                                              |
//! | **[`InFilt`]**            | Input with filter                                           |
//! | **[`InPu`]**              | Input with pull-up                                          |
//! | **[`InPuFilt`]**          | Input with pull-up and filter                               |
//! | **[`InPd`]**              | Input with pull-down                                        |
//! | **[`InPdFilt`]**          | Input with pull-down and filter                             |
//! |                           |                                                             |
//! | **[`OutPp`]**             | Output push-pull                                            |
//! | **[`OutOs`]**             | Output open source                                          |
//! | **[`OutOsPd`]**           | Output open source, pull-down                               |
//! | **[`OutOd`]**             | Output open drain                                           |
//! | **[`OutOdFilt`]**         | Output open drain with filter                               |
//! | **[`OutOdPu`]**           | Output open drain pull-up                                   |
//! | **[`OutOdPuFilt`]**       | Output open drain pull-up with filter                       |
//! |                           |                                                             |
//! | **[`OutPpAlt`]**          | Alternate Output push-pull                                  |
//! | **[`OutOdAlt`]**          | Alternate Output open drain                                 |
//! | **[`OutOdFiltAlt`]**      | Alternate Output open drain with filter                     |
//! | **[`OutOdPuAlt`]**        | Alternate Output open drain pull-up                         |
//! | **[`OutOdPuFiltAlt`]**    | Alternate Output open drain pull-up with filter             |
//!
//! ```rust,no_run
//! // The return type of the closure can be omitted with `_` when supplying the generic type parameters
//! let state = led0.with_mode::<InPuFilt, _>(|input_pin| input_pin.is_high().unwrap());
//!
//! // Same as above, but more verbose
//! let state: bool = led0.with_mode::<InPuFilt, bool>(|input_pin| input_pin.is_high().unwrap());
//! ```
//!

#[cfg(feature = "use_debug_pins")]
pub use crate::gpio::debug::DebugPinsEnabled;
use crate::gpio::dynamic::DynamicMode;
pub use crate::gpio::{
    pin::{
        mode::{
            Analog, Disabled, DisabledPu, InFilt, InFloat, InPd, InPdFilt, InPu, InPuFilt, OutOd,
            OutOdAlt, OutOdFilt, OutOdFiltAlt, OutOdPu, OutOdPuAlt, OutOdPuFilt, OutOdPuFiltAlt,
            OutOs, OutOsPd, OutPp, OutPpAlt,
        },
        Pin,
    },
    port::Port,
};
use embedded_hal::digital::{self, ErrorKind};

pub mod debug;
pub mod dynamic;
pub mod erased;
pub mod pin;
pub mod port;

/// Gpio ports and their pins
#[derive(Debug)]
pub struct Gpio {
    /// Port `A` configs for the entire port
    pub port_a: Port<'A'>,
    /// Port `B` configs for the entire port
    pub port_b: Port<'B'>,
    /// Port `C` configs for the entire port
    pub port_c: Port<'C'>,
    /// Port `D` configs for the entire port
    pub port_d: Port<'D'>,
    /// Port `F` configs for the entire port
    pub port_f: Port<'F'>,

    /// Port `A` pin `0`
    pub pa0: Pin<'A', 0, Disabled>,
    /// Port `A` pin `1`
    pub pa1: Pin<'A', 1, Disabled>,
    /// Port `A` pin `2`
    #[cfg(feature = "qfn48")]
    pub pa2: Pin<'A', 2, Disabled>,
    /// Port `A` pin `3`
    #[cfg(feature = "qfn48")]
    pub pa3: Pin<'A', 3, Disabled>,
    /// Port `A` pin `4`
    #[cfg(feature = "qfn48")]
    pub pa4: Pin<'A', 4, Disabled>,
    /// Port `A` pin `5`
    #[cfg(feature = "qfn48")]
    pub pa5: Pin<'A', 5, Disabled>,
    /// Port `B` pin `11`
    pub pb11: Pin<'B', 11, Disabled>,
    /// Port `B` pin `12`
    pub pb12: Pin<'B', 12, Disabled>,
    /// Port `B` pin `13`
    pub pb13: Pin<'B', 13, Disabled>,
    /// Port `B` pin `14`
    pub pb14: Pin<'B', 14, Disabled>,
    /// Port `B` pin `15`
    pub pb15: Pin<'B', 15, Disabled>,
    /// Port `C` pin `6`
    #[cfg(feature = "qfn48")]
    pub pc6: Pin<'C', 6, Disabled>,
    /// Port `C` pin `7`
    #[cfg(any(feature = "qfn32", feature = "qfn48"))]
    pub pc7: Pin<'C', 7, Disabled>,
    /// Port `C` pin `8`
    #[cfg(any(feature = "qfn32", feature = "qfn48"))]
    pub pc8: Pin<'C', 8, Disabled>,
    /// Port `C` pin `9`
    #[cfg(any(feature = "qfn32", feature = "qfn48"))]
    pub pc9: Pin<'C', 9, Disabled>,
    /// Port `C` pin `10`
    pub pc10: Pin<'C', 10, Disabled>,
    /// Port `C` pin `11`
    pub pc11: Pin<'C', 11, Disabled>,
    /// Port `D` pin `9`
    pub pd9: Pin<'D', 9, Disabled>,
    /// Port `D` pin `10`
    pub pd10: Pin<'D', 10, Disabled>,
    /// Port `D` pin `11`
    pub pd11: Pin<'D', 11, Disabled>,
    /// Port `D` pin `12`
    pub pd12: Pin<'D', 12, Disabled>,
    /// Port `D` pin `13`
    pub pd13: Pin<'D', 13, Disabled>,
    /// Port `D` pin `14`
    pub pd14: Pin<'D', 14, Disabled>,
    /// Port `D` pin `15`
    pub pd15: Pin<'D', 15, Disabled>,
    /// Port `F` debug pins: `0`, `1`, `2`, `3`
    #[cfg(feature = "use_debug_pins")]
    pub debug_pins: DebugPinsEnabled,
    /// Port `F` pin `4`
    #[cfg(any(feature = "qfn32", feature = "qfn48"))]
    pub pf4: Pin<'F', 4, Disabled>,
    /// Port `F` pin `5`
    #[cfg(feature = "qfn48")]
    pub pf5: Pin<'F', 5, Disabled>,
    /// Port `F` pin `6`
    #[cfg(feature = "qfn48")]
    pub pf6: Pin<'F', 6, Disabled>,
    /// Port `F` pin `7`
    #[cfg(feature = "qfn48")]
    pub pf7: Pin<'F', 7, Disabled>,

    /// GPIO PAC peripheral
    gpio_p: crate::pac::Gpio,
}

impl Gpio {
    /// Create the Gpio HAL driver consuming the PAC peripheral
    pub fn new(gpio_p: crate::pac::Gpio) -> Self {
        let mut gpio = Self {
            port_a: Port::new(),
            port_b: Port::new(),
            port_c: Port::new(),
            port_d: Port::new(),
            port_f: Port::new(),

            pa0: Pin::new(),
            pa1: Pin::new(),
            #[cfg(feature = "qfn48")]
            pa2: Pin::new(),
            #[cfg(feature = "qfn48")]
            pa3: Pin::new(),
            #[cfg(feature = "qfn48")]
            pa4: Pin::new(),
            #[cfg(feature = "qfn48")]
            pa5: Pin::new(),
            pb11: Pin::new(),
            pb12: Pin::new(),
            pb13: Pin::new(),
            pb14: Pin::new(),
            pb15: Pin::new(),
            #[cfg(feature = "qfn48")]
            pc6: Pin::new(),
            #[cfg(any(feature = "qfn32", feature = "qfn48"))]
            pc7: Pin::new(),
            #[cfg(any(feature = "qfn32", feature = "qfn48"))]
            pc8: Pin::new(),
            #[cfg(any(feature = "qfn32", feature = "qfn48"))]
            pc9: Pin::new(),
            pc10: Pin::new(),
            pc11: Pin::new(),
            pd9: Pin::new(),
            pd10: Pin::new(),
            pd11: Pin::new(),
            pd12: Pin::new(),
            pd13: Pin::new(),
            pd14: Pin::new(),
            pd15: Pin::new(),
            #[cfg(any(feature = "qfn32", feature = "qfn48"))]
            pf4: Pin::new(),
            #[cfg(feature = "qfn48")]
            pf5: Pin::new(),
            #[cfg(feature = "qfn48")]
            pf6: Pin::new(),
            #[cfg(feature = "qfn48")]
            pf7: Pin::new(),
            #[cfg(feature = "use_debug_pins")]
            debug_pins: DebugPinsEnabled::new(),

            gpio_p,
        };

        gpio.disable_clock();
        gpio.reset();
        gpio.enable_clock();

        gpio
    }

    /// Reset the GPIO to a known state
    fn reset(&mut self) {
        self.port_a.reset();
        self.port_b.reset();
        self.port_c.reset();
        self.port_d.reset();
        self.port_f.reset();

        self.gpio_p.em4wuen().reset();
        self.gpio_p.extifall().reset();
        self.gpio_p.extilevel().reset();
        self.gpio_p.extipinselh().reset();
        self.gpio_p.extipinsell().reset();
        self.gpio_p.extipselh().reset();
        self.gpio_p.extipsell().reset();
        self.gpio_p.ien().reset();
        self.gpio_p.ifc().reset();
        self.gpio_p.ifs().reset();
        self.gpio_p.insense().reset();
        self.gpio_p.lock().reset();
        self.gpio_p.routeloc0().reset();
        self.gpio_p.routepen().reset();
    }

    /// Enable clock for GPIO peripheral
    fn enable_clock(&mut self) {
        let cmu = unsafe { crate::pac::Cmu::steal() };

        // Enable GPIO clock
        cmu.hfbusclken0().modify(|_, w| w.gpio().set_bit());
    }

    /// Disable clock for GPIO peripheral
    fn disable_clock(&mut self) {
        let cmu = unsafe { crate::pac::Cmu::steal() };

        // Disable GPIO clock
        cmu.hfbusclken0().modify(|_, w| w.gpio().clear_bit());
    }
}

/// Check if the GPIO peripheral's clock is enabled
pub(crate) fn is_enabled() -> bool {
    let cmu = unsafe { crate::pac::Cmu::steal() };
    cmu.hfbusclken0().read().gpio().bit_is_set()
}

/// Gpio module errors
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GpioError {
    /// GPIO peripheral is disabled
    GpioDisabled,

    /// Pin level could not be read vecause Data In Disable is enablet for entire port
    DataInDisabled,

    /// Dynamic Pin mode does not support the operation requested
    InvalidMode(DynamicMode),

    /// Conversion of the given u8 to a [`port::DriveSlewRate`] failed
    InvalidSlewRate(u8),

    /// Failed to disable debug pins
    DebugPinsEnabled,

    /// Failed to convert a literal representation of port id to a [`port::PortId`]
    InvalidPortId(u8),

    /// Failed to convert a literal representation of port id to a [`port::PortId`]
    InvalidPortIdLabel(char),
}

impl embedded_hal::digital::Error for GpioError {
    fn kind(&self) -> digital::ErrorKind {
        match self {
            GpioError::GpioDisabled => ErrorKind::Other,
            GpioError::DataInDisabled => ErrorKind::Other,
            GpioError::InvalidMode(_) => ErrorKind::Other,
            GpioError::InvalidSlewRate(_) => ErrorKind::Other,
            GpioError::DebugPinsEnabled => ErrorKind::Other,
            GpioError::InvalidPortId(_) => ErrorKind::Other,
            GpioError::InvalidPortIdLabel(_) => ErrorKind::Other,
        }
    }
}
