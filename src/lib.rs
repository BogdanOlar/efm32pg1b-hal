//! efm32pg1b-hal
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
//!
#![no_std]

pub use efm32pg1b_pac as pac;

pub mod cmu;
pub mod gpio;
pub mod timer;
pub mod timer_le;
pub mod usart;

mod sealed {
    /// Sealed (typestate) marker trait for singleton types.
    /// Used to ensure that certain types may not be instantiated outside this crate.
    pub trait Sealed {}
}

pub(crate) use sealed::Sealed;

pub mod prelude {
    pub use crate::{
        cmu::CmuExt,
        gpio::{
            pin::mode::{
                Analog, Disabled, DisabledPu, InFilt, InFloat, InPd, InPdFilt, InPu, InPuFilt,
                OutOd, OutOdAlt, OutOdFilt, OutOdFiltAlt, OutOdPu, OutOdPuAlt, OutOdPuFilt,
                OutOdPuFiltAlt, OutOs, OutOsPd, OutPp, OutPpAlt,
            },
            port::{DataInCtrl, DriveStrength},
            Gpio, GpioError,
        },
        usart::{
            spi::{Spi, SpiError},
            Usart, UsartBuild,
        },
    };
    pub use efm32pg1b_pac as pac;
    pub use embedded_hal::{
        delay::DelayNs,
        digital::{InputPin, OutputPin, PinState, StatefulOutputPin},
        pwm::SetDutyCycle,
        spi::{self, SpiBus},
    };
    pub use fugit::RateExtU32;
}

// fn stripped_type_name<T>() -> &'static str {
//     let s = core::any::type_name::<T>();
//     let s = s.split("<").next().unwrap();
//     let p = s.split("::");
//     p.last().unwrap()
// }
