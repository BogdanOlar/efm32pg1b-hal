//! Build with `cargo build --example gpio`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;
use efm32pg1b_pac as pac;
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    let core_p = cortex_m::Peripherals::take().unwrap();
    let mut p = pac::Peripherals::take().unwrap();

    let gpio = p.gpio.split();

    gpio.port_f.set_drive_strength(false);
    gpio.port_f.set_drive_strength_alt(true);

    // Ups, don't do this until errors are implemented!
    // gpio.port_f.set_din_dis(true);

    let mut led0 = gpio.pf4.into_output().with_push_pull().build();
    let mut led1 = gpio.pf5.into_output_alt().with_push_pull().build();
    let mut button0 = gpio.pf6.into_input().build();
    let mut button1 = gpio.pf7.into_input().build();

    let mut btn0_prev = true;
    let mut btn1_prev = true;

    loop {
        let btn0_cur = button0.is_high().unwrap();
        if btn0_prev != btn0_cur {
            defmt::println!("btn0: {}", &btn0_cur);

            led0.toggle().unwrap();
            btn0_prev = btn0_cur;
        }

        let btn1_cur = button1.is_high().unwrap();

        if btn1_prev != btn1_cur {
            defmt::println!("btn1: {}", &btn1_cur);

            led1.toggle().unwrap();
            btn1_prev = btn1_cur;
        }
    }
}
