#![no_std]

pub use efm32pg1b_pac as pac;
pub use embedded_hal as hal;

pub mod cmu;
pub mod gpio;
pub mod usart;

pub mod prelude {
    pub use crate::cmu::{Clocks, CmuExt};
    pub use crate::gpio::{DataInCtrl, DriveStrengthCtrl, GpioExt};
    pub use crate::usart::UsartSpiExt;
    pub use efm32pg1b_pac as pac;
    pub use embedded_hal::digital::{InputPin, OutputPin, PinState, StatefulOutputPin};
    pub use embedded_hal::spi::SpiBus;
}

fn stripped_type_name<T>() -> &'static str {
    let s = core::any::type_name::<T>();
    let p = s.split("::");
    p.last().unwrap()
}
