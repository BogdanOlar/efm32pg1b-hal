//! TODO: Crate documentation
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
#![no_std]

pub use efm32pg1b_pac as pac;

pub mod cmu;
pub mod gpio;
pub mod spi;

pub mod prelude {
    pub use crate::{
        cmu::{Clocks, CmuExt},
        gpio::{DataInCtrl, DriveStrengthCtrl, GpioExt},
        spi::UsartSpiExt,
    };
    pub use efm32pg1b_pac as pac;
    pub use embedded_hal::{
        digital::{InputPin, OutputPin, PinState, StatefulOutputPin},
        spi::SpiBus,
    };
    pub use fugit::RateExtU32;
}

fn stripped_type_name<T>() -> &'static str {
    let s = core::any::type_name::<T>();
    let p = s.split("::");
    p.last().unwrap()
}
