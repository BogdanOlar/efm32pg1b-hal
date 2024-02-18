use core::{fmt, marker::PhantomData};

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Self::Parts;
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
pub struct Disabled;

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

/// Input mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Input;

#[doc = r" GPIO"]
pub mod gpio {
    use crate::pac::{self, GPIO};

    #[doc = r" GPIO parts"]
    pub struct Parts {
        #[doc = r" Pin F4"]
        pub pf4: PF4,
        #[doc = r" Pin F4"]
        pub pf5: PF5,
    }

    impl super::GpioExt for GPIO {
        type Parts = Parts;
        fn split(self) -> Parts {
            // enable GPIO
            unsafe {
                pac::CMU::steal()
                    .hfbusclken0()
                    .write(|w| w.gpio().set_bit());
            }

            Parts {
                pf4: PF4::new(),
                pf5: PF5::new(),
            }
        }
    }

    #[doc = stringify!(PF4)]
    #[doc = " pin"]
    pub type PF4<MODE = super::Disabled> = super::Pin<'F', 4, MODE>;

    #[doc = stringify!(PF5)]
    #[doc = " pin"]
    pub type PF5<MODE = super::Disabled> = super::Pin<'F', 5, MODE>;
}
pub use gpio::PF4;
