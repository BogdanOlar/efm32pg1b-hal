//! Build with `cargo build --example spi --features="defmt"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

use fugit::RateExtU32;
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();

    let p = pac::Peripherals::take().unwrap();

    let clocks = p.cmu.split();

    let gpio = p.gpio.split();

    let mut tx = gpio.pc6.into_output().with_push_pull().build();
    let mut rx = gpio.pc7.into_input().with_filter().build();
    let mut clk = gpio.pc8.into_output().with_push_pull().build();
    let mut cs = gpio.pd14.into_output().with_push_pull().build();

    let mut disp_com = gpio.pd13.into_output().with_push_pull().build();
    let mut disp_enable = gpio.pd15.into_output().with_push_pull().build();
    let mut btn0 = gpio.pf6.into_input().build();

    let mut spi = p.usart1.into_spi_bus(clk, tx, rx);
    let buf = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

    let br = spi.set_baudrate(0.MHz(), &clocks);
    defmt::println!("br: {}", br);

    let br = spi.set_baudrate(2.MHz(), &clocks);
    let ret_w = spi.write(&buf);
    defmt::println!("br: {}, ret_w: {}", br, ret_w);

    let br = spi.set_baudrate(1.MHz(), &clocks);
    let ret_w = spi.write(&buf);
    defmt::println!("br: {}, ret_w: {}", br, ret_w);

    let br = spi.set_baudrate(br.unwrap(), &clocks);
    let ret_w = spi.write(&buf);
    defmt::println!("br: {}, ret_w: {}", br, ret_w);

    defmt::println!("SPI!");

    loop {}
}
