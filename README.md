# efm32pg1b-hal
Hardware abstraction layer (HAL) for [Silicon Labs EFM32PG1B](https://www.silabs.com/mcu/32-bit/efm32-pearl-gecko/device.EFM32PG1B200F256GM48) microcontrollers

The [efm32pg1b-pac](https://github.com/BogdanOlar/efm32pg1b-pac) crate provides the register definitions and is re-exported as `pac` by this crate.

This crate implements [embedded-hal v1.0.0](https://github.com/rust-embedded/embedded-hal)

## Roadmap

- CMU: Clock Management Unit
    - [x] Basic implementation, can return the default [`crate::cmu::Clocks`]
    - [x] Handle selection of clock sources
    - [x] Handle prescalers to change clocks frequency
    - [ ] Handle Low Energy modes
    - [ ] Interrupts?

- SYSTICK:
    - [ ] [`embedded-hal`] traits:
        - [ ] `embedded_hal::delay::DelayNs`
        - [ ] `embedded_hal::pwm::SetDutyCycle` ?
    - [ ] Interrupts

- GPIO:
    - [x] Basic implementation
    - [x] [`embedded-hal`] traits:
        - [x] `embedded_hal::digital::InputPin`
        - [x] `embedded_hal::digital::OutputPin`
        - [x] `embedded_hal::digital::StatefulOutputPin`
    - [ ] Cargo features to differentiate between MCU HW packages which specify which pins are available
    - [ ] Interrupts

- SPI:
    - [x] Basic implementation, implements blocking master operations
    - [x] Pin constraints for alternate functions related to `Usart` in Synchronous mode
    - [ ] [`embedded-hal`] traits:
        - [x] `embedded_hal::spi::SpiBus`
        - [ ] `embedded_hal::spi::SpiDevice`
    - [ ] Some sort of SPI Manager/Server which arbitrates between of possibly different `embedded_hal::spi::SpiDevice`
    - [ ] Some sort of `SpiDeviceConfig` for each `SpiDevice`, which specifies the SPI parameters (Mode, Baudrate,
          Chip Select polarity) that each `embedded_hal::spi::SpiDevice` needs to be set while using the Spi Bus
    - [ ] Interrupts
    - [ ] Dma channel operation

- TIMER:
    - [x] [`embedded-hal`] traits:
        - [x] `embedded_hal::delay::DelayNs`
        - [x] `embedded_hal::pwm::SetDutyCycle`
    - [ ] Interrupts

- TBD

## Documentation

Vendor supplied documents:
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
