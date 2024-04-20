#![no_std]

pub use efm32pg1b_pac as pac;
pub use embedded_hal as hal;

pub mod gpio;

pub mod prelude {
    pub use crate::gpio::GpioExt;
    pub use embedded_hal::digital::{InputPin, OutputPin, PinState, StatefulOutputPin};
}

fn stripped_type_name<T>() -> &'static str {
    let s = core::any::type_name::<T>();
    let p = s.split("::");
    p.last().unwrap()
}
