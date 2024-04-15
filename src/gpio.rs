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
    use crate::pac::{self, Gpio};

    use super::{Disabled, Input, Output, Pin};

    #[doc = r" GPIO parts"]
    pub struct GpioParts {
        #[doc = r" Pin F5"]
        pub pf5: PF5,
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
                pf5: PF5::new(),
                pf7: PF7::new(),
            }
        }
    }

    #[doc = stringify!(PF5)]
    #[doc = " pin"]
    pub type PF5<MODE = Disabled> = Pin<'F', 5, MODE>;

    #[doc = stringify!(PF7)]
    #[doc = " pin"]
    pub type PF7<MODE = Disabled> = Pin<'F', 7, MODE>;

    impl Pin<'F', 5, Disabled> {
        pub fn into_output(self) -> Pin<'F', 5, Output> {
            let p = unsafe { Gpio::steal() };

            p.port_f().model().modify(|_, w| w.mode5().pushpull());

            Pin::new()
        }
    }

    impl Pin<'F', 5, Output> {
        pub fn set_high(&mut self) {
            let p = unsafe { Gpio::steal() };
            p.port_f().dout().modify(|_, w| w.dout5().set_bit());
        }

        pub fn set_low(&mut self) {
            let p = unsafe { Gpio::steal() };
            p.port_f().dout().modify(|_, w| w.dout5().clear_bit());
        }
    }

    impl Pin<'F', 7, Disabled> {
        pub fn into_input(self) -> Pin<'F', 7, Input> {
            let p = unsafe { Gpio::steal() };
            // Set port pin mode to input
            p.port_f().model().modify(|_, w| w.mode7().input());
            // Disable port pin filter
            p.port_f().dout().modify(|_, w| w.dout7().clear_bit());

            Pin::new()
        }
    }

    impl Pin<'F', 7, Input> {
        pub fn is_high(&self) -> bool {
            let p = unsafe { Gpio::steal() };
            p.port_f().din().read().din7().bit_is_set()
        }
    }
}
pub use gpio::{PF5, PF7};
