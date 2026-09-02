//! Build with `cargo build --example spi --features="defmt qfn48"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt::println;
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();

    let p = pac::Peripherals::take().unwrap();

    let gpio = Gpio::new(p.gpio);

    let tx = gpio.pc6.into_mode::<OutPp>();
    let rx = gpio.pc7.into_mode::<InFilt>();
    let clk = gpio.pc8.into_mode::<OutPp>();

    let mut spi = Spi::new(
        SpiPins::new(p.usart0, clk, tx, rx),
        &Config::new(spi::MODE_2, 0),
    );
    let write_orig = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    let mut write = write_orig;
    let mut read1 = [0; 5];
    let mut read2 = [0; 15];

    // 10 MHz: divider N = fHFPERCLK/(2*br) - 1 = 19e6/20e6 - 1 = 0 (clamped), actual br = 9.5 MHz

    spi.set_divider(0);

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    write = write_orig;

    // 1 MHz: divider N = 19e6/2e6 - 1 = 8, actual br = 19e6/(2*9) = 1.0555 MHz

    spi.set_divider(8);

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    write = write_orig;

    // 1 kHz: divider N = 19e6/2000 - 1 = 9499, actual br = 1000 Hz

    spi.set_divider(9499);

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    write = write_orig;

    // 1 Hz: divider N = 19e6/2 - 1 = 9_499_999

    spi.set_divider(9_499_999);
    // assert_eq!(br, 1); // FIXME: This is wrong. The actual br is about 316 Hz

    let ret_w = spi.write(&write);
    println!("\t ret_w: \t {}, {}", ret_w, write);

    let ret_tr1 = spi.transfer(&mut read1, &write);
    println!("\t ret_tr1: \t {}, {}, {}", ret_tr1, write, read1);

    let ret_tr2 = spi.transfer(&mut read2, &write);
    println!("\t ret_tr2: \t {}, {}, {}", ret_tr2, write, read2);

    let ret_trip = spi.transfer_in_place(&mut write);
    println!("\t ret_trip: \t {}, {}", ret_trip, write);
    // write = write_orig;

    println!("SPI: {}", &spi);

    // let (usart, clk, tx, rx) = spi.free();
    // println!("SPI Freed. Returned:");
    // println!("\t usart: {}", usart);
    // println!("\t clk: {}", clk);
    // println!("\t tx: {}", tx);
    // println!("\t rx: {}", rx);

    loop {}
}
