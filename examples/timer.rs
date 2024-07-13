//! Build with `cargo build --example timer --features="defmt"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

use embedded_hal::delay::DelayNs;
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
    let clocks = p.cmu.split();
    let timer = p.timer0.new(TimerDivider::Div1);
    let (tim0ch0, _tim0ch1, _tim0ch2, _tim0ch3) = timer.split();

    let mut delayer = tim0ch0.into_delay(&clocks);
    let mut seconds: u32 = 0;
    loop {
        if seconds > 10 {
            seconds = 0;
        } else {
            seconds += 2;
        }

        println!("Delay {} seconds", seconds);
        delayer.delay_ms(seconds * 1_000);
    }
}
