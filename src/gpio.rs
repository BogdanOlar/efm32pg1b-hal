use core::{fmt, marker::PhantomData};
use efm32pg1b_pac::{
    gpio::{port_a::model::MODE0, PortA},
    Gpio,
};
use embedded_hal::digital::{self, ErrorKind, ErrorType, InputPin, OutputPin, StatefulOutputPin};

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and enables GPIO
    fn split(self) -> Self::Parts;
}

/// Generic port type
///
/// - `P` is port name: `A` for GPIOA, `B` for GPIOB, etc.
pub struct Port<const P: char> {}

/// Generic pin type
///
/// - `MODE` is one of the pin modes (see [Modes](crate::gpio#modes) section).
/// - `P` is port name: `A` for GPIOA, `B` for GPIOB, etc.
/// - `N` is pin number: from `0` to `15`.
pub struct Pin<const P: char, const N: u8, MODE> {
    _mode: PhantomData<MODE>,
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
    const fn new() -> Self {
        Self { _mode: PhantomData }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GpioError {
    /// Returned when an operation which reads the Data In value for a Pin cannot be completed because the
    /// Data In Disabled port configuration is set
    DataInDisabled,

    /// Invalid pin configuration
    InvalidConfig,
}

impl embedded_hal::digital::Error for GpioError {
    fn kind(&self) -> digital::ErrorKind {
        match self {
            GpioError::DataInDisabled => ErrorKind::Other,
            GpioError::InvalidConfig => ErrorKind::Other,
        }
    }
}

// Implementations for `Pin` to be used for `embedded-hal` traits
impl<const P: char, const N: u8, MODE> ErrorType for Pin<P, N, MODE> {
    type Error = GpioError;
}

impl<const P: char, const N: u8, MODE> fmt::Debug for Pin<P, N, MODE> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "P{}{}<{}>",
            P,
            N,
            crate::stripped_type_name::<MODE>()
        ))
    }
}

#[cfg(feature = "defmt")]
impl<const P: char, const N: u8, MODE> defmt::Format for Pin<P, N, MODE> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "P{}{}<{}>", P, N, crate::stripped_type_name::<MODE>());
    }
}

/// Id, port and mode for any pin
pub trait PinExt {
    /// Current pin mode
    type Mode;
    /// Pin number
    fn pin_id(&self) -> u8;
    /// Port number starting from 0
    fn port_id(&self) -> u8;
}

impl<const P: char, const N: u8, MODE> PinExt for Pin<P, N, MODE> {
    type Mode = MODE;

    #[inline(always)]
    fn pin_id(&self) -> u8 {
        N
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        P as u8 - b'A'
    }
}

/// Disabled pin
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Disabled;

/// SWD Debug pin
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Swd<SwdMode> {
    _swd_mode: PhantomData<SwdMode>,
}

/// SwdMode (type state): SWD Clock
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SwdClk;

/// SwdMode (type state): SWD I/O
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SwdDio;

/// SwdMode (type state): TDO/SWO
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SwdTdo;

/// SwdMode (type state): TDI
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SwdTdi;

/// Input pin
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Input;

/// Output pin which uses either the `Necessary` or the `Alternate` output port control configs
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Output<OutMode> {
    _out_mode: PhantomData<OutMode>,
}

/// OutMode (type state): "Necessary" (as opposed to "Alternate")
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Necessary;

/// OutMode (type state): "Alternate" (as opposed to "Necessary")
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Alternate;

/// Input mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InputBuilder<InputMode, FilterMode> {
    _mode_input: PhantomData<InputMode>,
    _mode_filter: PhantomData<FilterMode>,
}

/// Output mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutputBuilder<OutputKind, OutputMode, FilterMode> {
    _mode_kind: PhantomData<OutputKind>,
    _mode_output: PhantomData<OutputMode>,
    _mode_filter: PhantomData<FilterMode>,
}

/// Output Alt mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutputAltBuilder<OutputKind, OutputMode, FilterMode> {
    _mode_kind: PhantomData<OutputKind>,
    _mode_output: PhantomData<OutputMode>,
    _mode_filter: PhantomData<FilterMode>,
}

/// InputMode variant (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Floating;

/// InputMode variant (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PullUp;

/// InputMode variant (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PullDown;

/// FilterMode variant (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NoFilter;

/// FilterMode variant (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Filter;

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PushPull;

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OpenSource;

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OpenDrain;

/// Initial Output mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutputSelect;

/// Initial Alternate Output mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutputAltSelect;

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
    /// Blanket implementation for `Pin` typestates: all `Pin`s can be set to `disabled`
    /// TODO: Maybe only allow this for built `Input` and `Output` pins
    pub fn into_disabled(self) -> Pin<P, N, Disabled> {
        Self::set_mode(MODE0::Disabled);
        Self::set_dout(false);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, Disabled> {
    /// Build a disabled `Pin` with PullUp
    pub fn with_pullup(self) -> Self {
        Self::set_mode(MODE0::Disabled);
        Self::set_dout(true);
        self
    }

    /// Builder for an input `Pin` with no PullUp and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn into_input(self) -> Pin<P, N, InputBuilder<Floating, NoFilter>> {
        Pin::new()
    }

    /// Builder for all output `Pin`s without an alternate config
    pub fn into_output(self) -> Pin<P, N, OutputSelect> {
        Pin::new()
    }

    /// Builder for all output `Pin`s with alternate configs
    pub fn into_output_alt(self) -> Pin<P, N, OutputAltSelect> {
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, InputBuilder<Floating, NoFilter>> {
    /// Builder for an input `Pin` with a Filter enabled
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, InputBuilder<Floating, Filter>> {
        Pin::new()
    }

    /// Builder for an input `Pin` with PullUp and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_pullup(self) -> Pin<P, N, InputBuilder<PullUp, NoFilter>> {
        Pin::new()
    }

    /// Builder for an input `Pin` with PullDown and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_pulldown(self) -> Pin<P, N, InputBuilder<PullDown, NoFilter>> {
        Pin::new()
    }

    /// Build an input `Pin` with no PullUp and no Filter
    pub fn build(self) -> Pin<P, N, Input> {
        Self::set_mode(MODE0::Input);
        Self::set_dout(false);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, InputBuilder<Floating, Filter>> {
    /// Build an input `Pin` with no PullUp and a Filter enabled
    pub fn build(self) -> Pin<P, N, Input> {
        Self::set_mode(MODE0::Input);
        Self::set_dout(true);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullUp, NoFilter>> {
    /// Builder for an input `Pin` with PullUp and a Filter enabled
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, InputBuilder<PullUp, Filter>> {
        Pin::new()
    }

    /// Build an input `Pin` with PullUp and no Filter
    pub fn build(self) -> Pin<P, N, Input> {
        Self::set_mode(MODE0::Inputpull);
        Self::set_dout(true);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullUp, Filter>> {
    /// Build an input `Pin` with PullUp and a Filter enabled
    pub fn build(self) -> Pin<P, N, Input> {
        Self::set_mode(MODE0::Inputpullfilter);
        Self::set_dout(true);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullDown, NoFilter>> {
    /// Builder for an input `Pin` with PullDown and a Filter enabled
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, InputBuilder<PullDown, Filter>> {
        Pin::new()
    }

    /// Build an input `Pin` with PullDown and no Filter
    pub fn build(self) -> Pin<P, N, Input> {
        Self::set_mode(MODE0::Inputpull);
        Self::set_dout(false);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullDown, Filter>> {
    /// Build an input `Pin` with PullDown and a Filter enabled
    pub fn build(self) -> Pin<P, N, Input> {
        Self::set_mode(MODE0::Inputpullfilter);
        Self::set_dout(false);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputSelect> {
    /// Builder for an output `Pin` with PushPull, Floating, and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_push_pull(self) -> Pin<P, N, OutputBuilder<PushPull, Floating, NoFilter>> {
        Pin::new()
    }

    /// Builder for an output `Pin` with OpenSource, Floating, and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_open_source(self) -> Pin<P, N, OutputBuilder<OpenSource, Floating, NoFilter>> {
        Pin::new()
    }

    /// Builder for an output `Pin` with OpenDrain, Floating, and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_open_drain(self) -> Pin<P, N, OutputBuilder<OpenDrain, Floating, NoFilter>> {
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<PushPull, Floating, NoFilter>> {
    /// Build an output `Pin` with PushPull
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Pushpull);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenSource, Floating, NoFilter>> {
    /// Builder for an output `Pin` with OpenSource, PullDown, and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_pulldown(self) -> Pin<P, N, OutputBuilder<OpenSource, PullDown, NoFilter>> {
        Pin::new()
    }

    /// Build an output `Pin` with OpenSource
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Wiredor);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenSource, PullDown, NoFilter>> {
    /// Build an output `Pin` with OpenSource and PullDown
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Wiredorpulldown);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, Floating, NoFilter>> {
    /// Builder for an output `Pin` with OpenDrain, Floating, and a Filter enabled
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, OutputBuilder<OpenDrain, Floating, Filter>> {
        Pin::new()
    }

    /// Builder for an output `Pin` with OpenDrain, PullUp, and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_pullup(self) -> Pin<P, N, OutputBuilder<OpenDrain, PullUp, NoFilter>> {
        Pin::new()
    }

    /// Build an output `Pin` with OpenDrain
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Wiredand);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, Floating, Filter>> {
    /// Build an output `Pin` with OpenDrain and a Filter enabled
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Wiredandfilter);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenSource, Floating, Filter>> {
    /// Build an output `Pin` with OpenSource and a Filter enabled
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, PullUp, NoFilter>> {
    /// Builder for an output `Pin` with OpenDrain, PullUp, and a Filter enabled
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, OutputBuilder<OpenDrain, PullUp, Filter>> {
        Pin::new()
    }

    /// Build an output `Pin` with OpenDrain and PullUp
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Wiredandpullup);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, PullUp, Filter>> {
    /// Build an output `Pin` with OpenDrain, a PullUp, and a Filter enabled
    pub fn build(self) -> Pin<P, N, Output<Necessary>> {
        Self::set_mode(MODE0::Wiredandpullupfilter);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputAltSelect> {
    /// Builder for an alternative output `Pin` with PushPull and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_push_pull(self) -> Pin<P, N, OutputAltBuilder<PushPull, Floating, NoFilter>> {
        Pin::new()
    }

    /// Builder for an alternative output `Pin` with OpenDrain and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_open_drain(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, NoFilter>> {
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<PushPull, Floating, NoFilter>> {
    // Build an alternate output `Pin` with PushPull, and no Filter
    pub fn build(self) -> Pin<P, N, Output<Alternate>> {
        Self::set_mode(MODE0::Pushpullalt);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, NoFilter>> {
    /// Builder for an alternative output `Pin` with OpenDrain and a Filter active
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, Filter>> {
        Pin::new()
    }

    /// Builder for an alternative output `Pin` with OpenDrain, PullUp and no Filter
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_pullup(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, NoFilter>> {
        Pin::new()
    }

    /// Build an alternate output `Pin` with OpenDrain and no Filter
    pub fn build(self) -> Pin<P, N, Output<Alternate>> {
        Self::set_mode(MODE0::Wiredandalt);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, Filter>> {
    /// Build an alternate output `Pin` with OpenDrain and a Filter active
    pub fn build(self) -> Pin<P, N, Output<Alternate>> {
        Self::set_mode(MODE0::Wiredandaltfilter);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, NoFilter>> {
    /// Builder for an alternative output `Pin` with OpenDrain, PullUp and a Filter active
    ///
    /// Note you need to call `build()` in order to finalize the pin builder and get a usable pin
    pub fn with_filter(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, Filter>> {
        Pin::new()
    }

    /// Build an alternate output `Pin` with OpenDrain, PullUp and no Filter
    pub fn build(self) -> Pin<P, N, Output<Alternate>> {
        Self::set_mode(MODE0::Wiredandaltpullup);
        Pin::new()
    }
}

impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, Filter>> {
    /// Build an alternate output `Pin` with OpenDrain, PullUp and a Filter active
    pub fn build(self) -> Pin<P, N, Output<Alternate>> {
        Self::set_mode(MODE0::Wiredandaltpullupfilter);
        Pin::new()
    }
}

/// `InputPin` implementation for trait from `embedded-hal`
impl<const P: char, const N: u8> InputPin for Pin<P, N, Input> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        if Port::<P>::new().din_dis() {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(Self::din())
        }
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        if Port::<P>::new().din_dis() {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(!Self::din())
        }
    }
}

/// `OutputPin` implementation for trait from `embedded-hal`
impl<const P: char, const N: u8, OUTMODE> OutputPin for Pin<P, N, Output<OUTMODE>> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Self::set_dout(false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Self::set_dout(true);
        Ok(())
    }
}

/// `StatefulOutputPin` implementation for trait from `embedded-hal`
impl<const P: char, const N: u8> StatefulOutputPin for Pin<P, N, Output<Necessary>> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        if Port::<P>::new().din_dis() {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(Self::din())
        }
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        if Port::<P>::new().din_dis() {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(!Self::din())
        }
    }
}

/// `StatefulOutputPin` (`Alternate` output mode) implementation for trait from `embedded-hal`
impl<const P: char, const N: u8> StatefulOutputPin for Pin<P, N, Output<Alternate>> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        if Port::<P>::new().din_dis_alt() {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(Self::din())
        }
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        if Port::<P>::new().din_dis_alt() {
            Err(GpioError::DataInDisabled)
        } else {
            Ok(!Self::din())
        }
    }
}

/// Get the memory mapped `PortA` reference corresponding to the port specified by the generic parameter `P`
///
/// Note: We're returning a `PortA` because all ports use the same struct (they have type aliases to this type)
const fn portx<const P: char>() -> &'static PortA {
    match P {
        'A' => unsafe { (*Gpio::ptr()).port_a() },
        'B' => unsafe { (*Gpio::ptr()).port_b() },
        'C' => unsafe { (*Gpio::ptr()).port_c() },
        'D' => unsafe { (*Gpio::ptr()).port_d() },
        'E' => unsafe { (*Gpio::ptr()).port_e() },
        'F' => unsafe { (*Gpio::ptr()).port_f() },
        _ => unreachable!(),
    }
}

/// Data In Control variants for `DIN_DIS` (and `ALT`) field in `GPIO_Px_CTRL` Port Control Register
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DataInCtrl {
    /// Data In is NOT disabled
    Enabled,
    /// Data In is disabled
    Disabled,
}

/// Drive current variants for `DRIVESTRENGTH` (and `ALT`) field in `GPIO_Px_CTRL` Port Control Register
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DriveStrengthCtrl {
    /// Drive strength 10 mA drive current
    Strong,
    /// Drive strength 10 mA drive current
    Weak,
}

impl<const P: char> Port<P> {
    /// Construct a new `Port` with the given generic parameter `P` identifier (`A` for GPIOA, `B` for GPIOB, etc.)
    const fn new() -> Self {
        Self {}
    }

    /// Get the Drive Strength setting of this port
    pub fn drive_strength(&self) -> DriveStrengthCtrl {
        match portx::<P>().ctrl().read().drive_strength().bit() {
            true => DriveStrengthCtrl::Weak,
            false => DriveStrengthCtrl::Strong,
        }
    }

    /// Get the Alternate Drive Strength setting of this port
    pub fn drive_strength_alt(&self) -> DriveStrengthCtrl {
        match portx::<P>().ctrl().read().drive_strength_alt().bit() {
            true => DriveStrengthCtrl::Weak,
            false => DriveStrengthCtrl::Strong,
        }
    }

    /// Set the Drive Strength setting of this port
    pub fn set_drive_strength(&self, drive_strength: DriveStrengthCtrl) {
        portx::<P>().ctrl().modify(|_, w| match drive_strength {
            DriveStrengthCtrl::Strong => w.drive_strength().clear_bit(),
            DriveStrengthCtrl::Weak => w.drive_strength().set_bit(),
        })
    }

    /// Set the Alternate Drive Strength setting of this port
    pub fn set_drive_strength_alt(&self, drive_strength: DriveStrengthCtrl) {
        portx::<P>().ctrl().modify(|_, w| match drive_strength {
            DriveStrengthCtrl::Strong => w.drive_strength().clear_bit(),
            DriveStrengthCtrl::Weak => w.drive_strength().set_bit(),
        })
    }

    /// Get the Slew Rate setting of this port. Higher values represent faster slewrates.
    ///
    /// Max value is `5`.
    pub fn slew_rate(&self) -> u8 {
        portx::<P>().ctrl().read().slew_rate().bits()
    }

    /// Get the Slew Rate setting of this port. Higher values represent faster slewrates.
    ///
    /// Max value is `5`.
    pub fn slew_rate_alt(&self) -> u8 {
        portx::<P>().ctrl().read().slew_rate_alt().bits()
    }

    /// Set the Slew Rate setting of this port. Higher values represent faster slewrates
    ///
    /// Note: `0 <= slew_rate <= 5`
    pub fn set_slew_rate(&self, slew_rate: u8) -> Result<(), GpioError> {
        match slew_rate {
            0..=5 => {
                portx::<P>()
                    .ctrl()
                    .modify(|_, w| unsafe { w.slew_rate().bits(slew_rate) });
                Ok(())
            }
            _ => Err(GpioError::InvalidConfig),
        }
    }

    /// Set the Alternate Slew Rate setting of this port. Higher values represent faster slewrates.
    ///
    /// Note: `0 <= slew_rate <= 5`
    pub fn set_slew_rate_alt(&self, slew_rate: u8) -> Result<(), GpioError> {
        match slew_rate {
            0..=5 => {
                portx::<P>()
                    .ctrl()
                    .modify(|_, w| unsafe { w.slew_rate_alt().bits(slew_rate) });
                Ok(())
            }
            _ => Err(GpioError::InvalidConfig),
        }
    }

    /// Get the Data In Disable setting of this port
    pub fn din_dis(&self) -> bool {
        portx::<P>().ctrl().read().din_dis().bit_is_set()
    }

    /// Get the Alternate Data In Disable setting of this port
    pub fn din_dis_alt(&self) -> bool {
        portx::<P>().ctrl().read().din_dis_alt().bit_is_set()
    }

    /// Set the Data In Disable setting of this port
    pub fn set_din_dis(&self, din_dis: DataInCtrl) {
        portx::<P>().ctrl().modify(|_, w| match din_dis {
            DataInCtrl::Enabled => w.din_dis().clear_bit(),
            DataInCtrl::Disabled => w.din_dis().set_bit(),
        })
    }

    /// Set the Alternate Data In Disable setting of this port
    pub fn set_din_dis_alt(&self, din_dis: DataInCtrl) {
        portx::<P>().ctrl().modify(|_, w| match din_dis {
            DataInCtrl::Enabled => w.din_dis_alt().clear_bit(),
            DataInCtrl::Disabled => w.din_dis_alt().set_bit(),
        })
    }
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
    /// Set the Mode for this `Pin`
    fn set_mode(iomode: MODE0) {
        match N {
            0..=7 => {
                portx::<P>().model().modify(|_, w| {
                    match N {
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
                portx::<P>().modeh().modify(|_, w| {
                    match N {
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

    /// Set the Data Out for this `Pin`. If the pin is configured as an input, the meaning of the field varies by Mode
    fn set_dout(state: bool) {
        // Set/clear filter
        portx::<P>().dout().modify(|r, w| match state {
            true => unsafe { w.pins_dout().bits(r.bits() as u16 | (1 << N)) },
            false => unsafe { w.pins_dout().bits(r.bits() as u16 & !(1u16 << N)) },
        });
    }

    /// Get the Data In bit for this `Pin`
    fn din() -> bool {
        (portx::<P>().din().read().bits() & (1u32 << N)) != 0
    }
}

#[doc = r" GPIO"]
pub mod gpio {
    use super::{Disabled, Pin, Port, Swd, SwdClk, SwdDio, SwdTdi, SwdTdo};
    use crate::pac::{self, Gpio};

    #[doc = r" GPIO parts"]
    pub struct GpioParts {
        #[doc = r" Port A configs for the entire port"]
        pub port_a: Port<'A'>,
        #[doc = r" Port B configs for the entire port"]
        pub port_b: Port<'B'>,
        #[doc = r" Port C configs for the entire port"]
        pub port_c: Port<'C'>,
        #[doc = r" Port D configs for the entire port"]
        pub port_d: Port<'D'>,
        #[doc = r" Port E configs for the entire port"]
        pub port_e: Port<'E'>,
        #[doc = r" Port F configs for the entire port"]
        pub port_f: Port<'F'>,

        pub pc6: PC6,
        pub pc7: PC7,
        pub pc8: PC8,
        pub pd13: PD13,
        pub pd14: PD14,
        pub pd15: PD15,

        #[doc = r" DEBUG Pin F0"]
        pub pf0: PF0,
        #[doc = r" DEBUG Pin F1"]
        pub pf1: PF1,
        #[doc = r" DEBUG Pin F2"]
        pub pf2: PF2,
        #[doc = r" DEBUG Pin F3"]
        pub pf3: PF3,

        #[doc = r" Pin F5"]
        pub pf4: PF4,
        #[doc = r" Pin F5"]
        pub pf5: PF5,
        #[doc = r" Pin F6"]
        pub pf6: PF6,
        #[doc = r" Pin F7"]
        pub pf7: PF7,
    }

    impl super::GpioExt for Gpio {
        type Parts = GpioParts;
        fn split(self) -> GpioParts {
            // Make sure to enable GPIO
            unsafe {
                pac::Cmu::steal()
                    .hfbusclken0()
                    .write(|w| w.gpio().set_bit());
            }

            GpioParts {
                port_a: Port::new(),
                port_b: Port::new(),
                port_c: Port::new(),
                port_d: Port::new(),
                port_e: Port::new(),
                port_f: Port::new(),

                pc6: PC6::new(),
                pc7: PC7::new(),
                pc8: PC8::new(),
                pd13: PD13::new(),
                pd14: PD14::new(),
                pd15: PD15::new(),

                pf0: PF0::new(),
                pf1: PF1::new(),
                pf2: PF2::new(),
                pf3: PF3::new(),
                pf4: PF4::new(),
                pf5: PF5::new(),
                pf6: PF6::new(),
                pf7: PF7::new(),
            }
        }
    }

    pub type PC6<MODE = Disabled> = Pin<'C', 6, MODE>;
    pub type PC7<MODE = Disabled> = Pin<'C', 7, MODE>;
    pub type PC8<MODE = Disabled> = Pin<'C', 8, MODE>;
    pub type PD13<MODE = Disabled> = Pin<'D', 13, MODE>;
    pub type PD14<MODE = Disabled> = Pin<'D', 14, MODE>;
    pub type PD15<MODE = Disabled> = Pin<'D', 15, MODE>;

    #[doc = stringify!(PF0)]
    #[doc = " DEBUG pin"]
    pub type PF0<MODE = Swd<SwdClk>> = Pin<'F', 0, MODE>;

    #[doc = stringify!(PF1)]
    #[doc = " DEBUG pin"]
    pub type PF1<MODE = Swd<SwdDio>> = Pin<'F', 1, MODE>;

    #[doc = stringify!(PF2)]
    #[doc = " DEBUG pin"]
    pub type PF2<MODE = Swd<SwdTdo>> = Pin<'F', 2, MODE>;

    #[doc = stringify!(PF3)]
    #[doc = " DEBUG pin"]
    pub type PF3<MODE = Swd<SwdTdi>> = Pin<'F', 3, MODE>;

    #[doc = stringify!(PF4)]
    #[doc = " pin"]
    pub type PF4<MODE = Disabled> = Pin<'F', 4, MODE>;

    #[doc = stringify!(PF5)]
    #[doc = " pin"]
    pub type PF5<MODE = Disabled> = Pin<'F', 5, MODE>;

    #[doc = stringify!(PF6)]
    #[doc = " pin"]
    pub type PF6<MODE = Disabled> = Pin<'F', 6, MODE>;

    #[doc = stringify!(PF7)]
    #[doc = " pin"]
    pub type PF7<MODE = Disabled> = Pin<'F', 7, MODE>;
}
pub use gpio::{PF5, PF7};
