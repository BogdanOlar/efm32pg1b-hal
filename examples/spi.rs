//! Build with `cargo build --example spi --features="defmt"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

use embedded_hal::spi::{self, Phase, Polarity};
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt::{assert_eq, println};
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();

    let p = pac::Peripherals::take().unwrap();

    let clocks = p.cmu.split();

    let gpio = p.gpio.split();

    let tx = gpio.pc6.into_output().with_push_pull().build();
    let rx = gpio.pc7.into_input().with_filter().build();
    let clk = gpio.pc8.into_output().with_push_pull().build();

    let mut spi = p.usart1.into_spi_bus(
        clk,
        tx,
        rx,
        spi::Mode {
            polarity: Polarity::IdleHigh,
            phase: Phase::CaptureOnFirstTransition,
        },
    );
    let write_orig = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    let mut write = write_orig;
    let mut read1 = [0; 5];
    let mut read2 = [0; 15];

    // 10.MHz()

    let br = spi.set_baudrate(10.MHz(), &clocks);
    println!("br: {}", br);
    assert_eq!(br.unwrap(), 9500000.Hz::<1, 1>());

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    write = write_orig;

    // 1.MHz()

    let br = spi.set_baudrate(1.MHz(), &clocks);
    println!("br: {}", br);
    assert_eq!(br.unwrap(), 1055555.Hz::<1, 1>());

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    write = write_orig;

    // 1.kHz()

    let br = spi.set_baudrate(1.kHz(), &clocks);
    println!("br: {}", br);
    assert_eq!(br.unwrap(), 1.kHz::<1, 1>());

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    write = write_orig;

    // 1.Hz()

    let br = spi.set_baudrate(1.Hz(), &clocks);
    println!("br: {}", br);
    assert_eq!(br.unwrap(), 1.Hz::<1, 1>()); // FIXME: This is wrong. The actual br is about 316 Hz

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    // write = write_orig;

    let (clk, tx, rx) = spi.destroy();

    println!("SPI Destroyed. Returned pins:");
    println!("\t clk: {}", clk);
    println!("\t tx: {}", tx);
    println!("\t rx: {}", rx);

    loop {}
}
