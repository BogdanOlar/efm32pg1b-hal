use crate::pac;
use core::{fmt, marker::PhantomData};

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and enables GPIO
    fn split(self) -> Self::Parts;
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

/// Input mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Input;

/// Output mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Output;

#[doc = r" GPIO"]
pub mod gpio {
    use pac::gpio::port_a::model::MODE0;

    use super::{Disabled, Input, Output, Pin};
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

    impl<const P: char, const N: u8> Pin<P, N, Disabled> {
        pub fn into_input(self) -> Pin<P, N, Input> {
            Self::set_mode(MODE0::Input);
            Self::dout(false);
            Pin::new()
        }

        pub fn into_output(self) -> Pin<P, N, Output> {
            Self::set_mode(MODE0::Pushpull);
            Pin::new()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, Input> {
        pub fn is_high(&self) -> bool {
            Self::din()
        }
    }

    impl<const P: char, const N: u8> Pin<P, N, Output> {
        pub fn set_high(&mut self) {
            Self::dout(true);
        }

        pub fn set_low(&mut self) {
            Self::dout(false);
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

        fn dout(state: bool) {
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
            port.dout().modify(|_, w| {
                let dout = match N {
                    0 => w.dout0(),
                    1 => w.dout1(),
                    2 => w.dout2(),
                    3 => w.dout3(),
                    4 => w.dout4(),
                    5 => w.dout5(),
                    6 => w.dout6(),
                    7 => w.dout7(),
                    8 => w.dout8(),
                    9 => w.dout9(),
                    10 => w.dout10(),
                    11 => w.dout11(),
                    12 => w.dout12(),
                    13 => w.dout13(),
                    14 => w.dout14(),
                    15 => w.dout15(),
                    _ => unreachable!(),
                };

                match state {
                    true => dout.set_bit(),
                    false => dout.clear_bit(),
                }
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

            (port.din().read().bits() & (1 << N)) != 0
        }
    }
}
pub use gpio::{PF5, PF7};
