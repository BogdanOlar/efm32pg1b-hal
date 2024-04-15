//! Build with `cargo build --example gpio`

#![no_main]
#![no_std]

use core::convert::TryInto;

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

    let button1 = gpio.pf7.into_input();
    let mut led1 = gpio.pf5.into_output();

    let mut btn1_prev = true;
    loop {
        // let din_reg_value = p.gpio.pf_din().read().bits();

        // if (din_reg_value & PF6_PIN_MASK) == 0 {
        //     p.gpio
        //         .pf_dout()
        //         .modify(|r, w| unsafe { w.bits(r.bits() | PF4_PIN_MASK) });
        // } else {
        //     p.gpio
        //         .pf_dout()
        //         .modify(|r, w| unsafe { w.bits(r.bits() & !PF4_PIN_MASK) });
        // }

        let btn1_cur = button1.is_high();

        if btn1_prev != btn1_cur {
            defmt::println!("btn1: {}", &btn1_cur);

            if btn1_cur {
                led1.set_low();
            } else {
                led1.set_high();
            }

            btn1_prev = btn1_cur;
        }
    }
}
