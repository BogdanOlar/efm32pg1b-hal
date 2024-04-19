use core::{convert::Infallible, fmt, marker::PhantomData};

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
pub struct Port<const P: char, MODE> {
    _mode: PhantomData<MODE>,
}

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

// Implementations for `Pin` to be used for `embedded-hal` traits
impl<const P: char, const N: u8, MODE> ErrorType for Pin<P, N, MODE> {
    type Error = Infallible;
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

/// Disabled pin mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Disabled;

/// Input pin mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Input;

/// Output pin mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Output;

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

/// Initial Output mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutputAltSelect;

#[doc = r" GPIO"]
pub mod gpio {
    use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
    use pac::gpio::port_a::model::MODE0;

    use super::{
        Disabled, Filter, Floating, Input, InputBuilder, NoFilter, OpenDrain, OpenSource, Output,
        OutputAltBuilder, OutputAltSelect, OutputBuilder, OutputSelect, Pin, PullDown, PullUp,
        PushPull,
    };
    use crate::pac::{self, Gpio};

    #[doc = r" GPIO parts"]
    pub struct GpioParts {
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
                pf4: PF4::new(),
                pf5: PF5::new(),
                pf6: PF6::new(),
                pf7: PF7::new(),
            }
        }
    }

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

    impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
        /// Blanket implementation for `Pin` typestates: all `Pin`s can be set to `disabled`
        pub fn into_disabled(self) -> Pin<P, N, Disabled> {
            Self::set_mode(MODE0::Disabled);
            Self::set_dout(false);
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, Disabled> {
        pub fn with_pullup(self) -> Self {
            Self::set_mode(MODE0::Disabled);
            Self::set_dout(true);
            self
        }

        pub fn into_input(self) -> Pin<P, N, InputBuilder<Floating, NoFilter>> {
            Self::set_mode(MODE0::Input);
            Self::set_dout(false);
            Pin::new()
        }

        pub fn into_output(self) -> Pin<P, N, OutputSelect> {
            Pin::new()
        }

        pub fn into_output_alt(self) -> Pin<P, N, OutputAltSelect> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, InputBuilder<Floating, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, InputBuilder<Floating, Filter>> {
            Self::set_mode(MODE0::Input);
            Self::set_dout(true);
            Pin::new()
        }

        pub fn with_pullup(self) -> Pin<P, N, InputBuilder<PullUp, NoFilter>> {
            Self::set_mode(MODE0::Inputpull);
            Self::set_dout(true);
            Pin::new()
        }

        pub fn with_pulldown(self) -> Pin<P, N, InputBuilder<PullDown, NoFilter>> {
            Self::set_mode(MODE0::Inputpull);
            Self::set_dout(false);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Input> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, InputBuilder<Floating, Filter>> {
        pub fn build(self) -> Pin<P, N, Input> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullUp, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, InputBuilder<PullUp, Filter>> {
            Self::set_mode(MODE0::Inputpullfilter);
            Self::set_dout(true);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Input> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullUp, Filter>> {
        pub fn build(self) -> Pin<P, N, Input> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullDown, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, InputBuilder<PullDown, Filter>> {
            Self::set_mode(MODE0::Inputpullfilter);
            Self::set_dout(false);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Input> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, InputBuilder<PullDown, Filter>> {
        pub fn build(self) -> Pin<P, N, Input> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputSelect> {
        pub fn with_push_pull(self) -> Pin<P, N, OutputBuilder<PushPull, Floating, NoFilter>> {
            Self::set_mode(MODE0::Pushpull);
            Pin::new()
        }

        pub fn with_open_source(self) -> Pin<P, N, OutputBuilder<OpenSource, Floating, NoFilter>> {
            Self::set_mode(MODE0::Wiredor);
            Pin::new()
        }

        pub fn with_open_drain(self) -> Pin<P, N, OutputBuilder<OpenDrain, Floating, NoFilter>> {
            Self::set_mode(MODE0::Wiredand);
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<PushPull, Floating, NoFilter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenSource, Floating, NoFilter>> {
        pub fn with_pulldown(self) -> Pin<P, N, OutputBuilder<OpenSource, PullDown, NoFilter>> {
            Self::set_mode(MODE0::Wiredorpulldown);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenSource, PullDown, NoFilter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, Floating, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, OutputBuilder<OpenDrain, Floating, Filter>> {
            Self::set_mode(MODE0::Wiredandfilter);
            Pin::new()
        }

        pub fn with_pullup(self) -> Pin<P, N, OutputBuilder<OpenDrain, PullUp, NoFilter>> {
            Self::set_mode(MODE0::Wiredandpullup);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, Floating, Filter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenSource, Floating, Filter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, PullUp, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, OutputBuilder<OpenDrain, PullUp, Filter>> {
            Self::set_mode(MODE0::Wiredandpullupfilter);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputBuilder<OpenDrain, PullUp, Filter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputAltSelect> {
        pub fn with_push_pull(self) -> Pin<P, N, OutputAltBuilder<PushPull, Floating, NoFilter>> {
            Self::set_mode(MODE0::Pushpullalt);
            Pin::new()
        }

        pub fn with_open_drain(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, NoFilter>> {
            Self::set_mode(MODE0::Wiredandalt);
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<PushPull, Floating, NoFilter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, Filter>> {
            Self::set_mode(MODE0::Wiredandaltfilter);
            Pin::new()
        }

        pub fn with_pullup(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, NoFilter>> {
            Self::set_mode(MODE0::Wiredandaltpullup);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, Floating, Filter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, NoFilter>> {
        pub fn with_filter(self) -> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, Filter>> {
            Self::set_mode(MODE0::Wiredandaltpullupfilter);
            Pin::new()
        }

        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, OutputAltBuilder<OpenDrain, PullUp, Filter>> {
        pub fn build(self) -> Pin<P, N, Output> {
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> InputPin for Pin<P, N, Input> {
        fn is_high(&mut self) -> Result<bool, Self::Error> {
            if Self::din_dis() {
                // TODO: Data in is disabled for port, so we should return an error here
                todo!()
            } else {
                Ok(Self::din())
            }
        }

        fn is_low(&mut self) -> Result<bool, Self::Error> {
            if Self::din_dis() {
                // TODO: Data in is disabled for port, so we should return an error here
                todo!()
            } else {
                Ok(!Self::din())
            }
        }
    }

    impl<const P: char, const N: u8> OutputPin for Pin<P, N, Output> {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Self::set_dout(false);
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Self::Error> {
            Self::set_dout(true);
            Ok(())
        }
    }

    impl<const P: char, const N: u8> StatefulOutputPin for Pin<P, N, Output> {
        fn is_set_high(&mut self) -> Result<bool, Self::Error> {
            if Self::din_dis() {
                // TODO: Data in is disabled for port, so we should return an error here
                todo!()
            } else {
                Ok(Self::din())
            }
        }

        fn is_set_low(&mut self) -> Result<bool, Self::Error> {
            if Self::din_dis() {
                // TODO: Data in is disabled for port, so we should return an error here
                todo!()
            } else {
                Ok(!Self::din())
            }
        }
    }

    impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
        fn set_mode(iomode: MODE0) {
            let port = match P {
                'A' => unsafe { (*Gpio::ptr()).port_a() },
                'B' => unsafe { (*Gpio::ptr()).port_b() },
                'C' => unsafe { (*Gpio::ptr()).port_c() },
                'D' => unsafe { (*Gpio::ptr()).port_d() },
                'E' => unsafe { (*Gpio::ptr()).port_e() },
                'F' => unsafe { (*Gpio::ptr()).port_f() },
                _ => unreachable!(),
            };

            match N {
                0..=7 => {
                    // Set port pin mode to input
                    port.model().modify(|_, w| {
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
                    // Set port pin mode to input
                    port.modeh().modify(|_, w| {
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

        fn set_dout(state: bool) {
            let port = match P {
                'A' => unsafe { (*Gpio::ptr()).port_a() },
                'B' => unsafe { (*Gpio::ptr()).port_b() },
                'C' => unsafe { (*Gpio::ptr()).port_c() },
                'D' => unsafe { (*Gpio::ptr()).port_d() },
                'E' => unsafe { (*Gpio::ptr()).port_e() },
                'F' => unsafe { (*Gpio::ptr()).port_f() },
                _ => unreachable!(),
            };

            // Set/clear filter
            port.dout().modify(|r, w| match state {
                true => unsafe { w.pins_dout().bits(r.bits() as u16 | (1 << N)) },
                false => unsafe { w.pins_dout().bits(r.bits() as u16 & !(1u16 << N)) },
            });
        }

        fn din() -> bool {
            let port = match P {
                'A' => unsafe { (*Gpio::ptr()).port_a() },
                'B' => unsafe { (*Gpio::ptr()).port_b() },
                'C' => unsafe { (*Gpio::ptr()).port_c() },
                'D' => unsafe { (*Gpio::ptr()).port_d() },
                'E' => unsafe { (*Gpio::ptr()).port_e() },
                'F' => unsafe { (*Gpio::ptr()).port_f() },
                _ => unreachable!(),
            };

            (port.din().read().bits() & (1u32 << N)) != 0
        }

        fn din_dis() -> bool {
            let port = match P {
                'A' => unsafe { (*Gpio::ptr()).port_a() },
                'B' => unsafe { (*Gpio::ptr()).port_b() },
                'C' => unsafe { (*Gpio::ptr()).port_c() },
                'D' => unsafe { (*Gpio::ptr()).port_d() },
                'E' => unsafe { (*Gpio::ptr()).port_e() },
                'F' => unsafe { (*Gpio::ptr()).port_f() },
                _ => unreachable!(),
            };

            port.ctrl().read().din_dis().bit_is_set()
        }
    }
}
use embedded_hal::digital::ErrorType;
pub use gpio::{PF5, PF7};
