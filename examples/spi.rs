//! Build with `cargo build --example spi --features="defmt qfn48"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::{
    cmu::CmuExt,
    gpio::{Gpio, InFilt, OutPp},
    pac,
    usart::{Usart, UsartBuild},
};
use embedded_hal::spi::{self, SpiBus};
use fugit::RateExtU32;
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

    let gpio = Gpio::new(p.gpio);

    let tx = gpio.pc6.into_mode::<OutPp>();
    let rx = gpio.pc7.into_mode::<InFilt>();
    let clk = gpio.pc8.into_mode::<OutPp>();

    let usart0 = Usart::new(p.usart0);
    let usart1 = Usart::new(p.usart1);

    // We're not going to use this
    let _usart1_p = usart1.free();

    let mut spi = usart0.into_spi_bus(clk, tx, rx, spi::MODE_2);
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
    // assert_eq!(br.unwrap(), 1.Hz::<1, 1>()); // FIXME: This is wrong. The actual br is about 316 Hz

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

    let (usart, clk, tx, rx) = spi.free();
    println!("SPI Freed. Returned:");
    println!("\t usart: {}", usart);
    println!("\t clk: {}", clk);
    println!("\t tx: {}", tx);
    println!("\t rx: {}", rx);

    loop {}
}
