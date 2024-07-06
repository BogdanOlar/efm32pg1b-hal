# efm32pg1b-hal
Hardware abstraction layer (HAL) for [Silicon Labs EFM32PG1B](https://www.silabs.com/mcu/32-bit/efm32-pearl-gecko/device.EFM32PG1B200F256GM48) microcontrollers

The [efm32pg1b-pac](https://github.com/BogdanOlar/efm32pg1b-pac) crate provides the register definitions and is re-exported as `pac` by this crate.

This crate implements [embedded-hal v1.0.0](https://github.com/rust-embedded/embedded-hal)

## Roadmap

- CMU: Clock Management Unit
    - [x] Basic implementation, can return the default [`Clocks`]
    - [ ] Handle prescalers to change clocks frequency
    - [ ] Handle Low Energy modes
    - [ ] Interrupts?
    - [ ] Unit tests ?

- GPIO:
    - [x] Basic implementation, implements [`embedded_hal::digital::InputPin`], ['embedded_hal::digital::OutputPin`] and [`embedded_hal::digital::StatefulOutputPin`] traits
    - [ ] Interrupts
    - [ ] Unit tests ?

- SPI:
    - [x] Basic implementation, implements blocking master operations
    - [x] Pin constraints for alternate functions related to `Usart` in Synchronous mode
    - [x] ['embedded_hal::spi::SpiBus`] trait implementation
    - [ ] ['embedded_hal::spi::SpiDevice`] trait implementation
    - [ ] Interrupts
    - [ ] Dma channel operation
    - [ ] Unit tests ?

- TIMER:
    -[ ] [`embedded-hal`] traits ?
    -[ ] Interrupts
    -[ ] PWM generation ?
    -[ ] Unit tests ?

- TBD

## Documentation

Additional vendor supplied documents:
- [Datasheet](https://www.silabs.com/documents/public/data-sheets/efm32pg1-datasheet.pdf)
- [Reference Manual](https://www.silabs.com/documents/public/reference-manuals/EFM32PG1-ReferenceManual.pdf)
- [Errata](https://www.silabs.com/documents/public/errata/efm32pg1-errata.pdf)
- [CMSIS Pack](https://www.keil.arm.com/devices/silicon-labs-efm32pg1b200f256gm48/processors/)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
