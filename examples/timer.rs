//! Build with `cargo build --example timer --features="defmt"`

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
    let clocks = p.cmu.split();
    let gpio = p.gpio.split();
    let mut pin_delay = gpio.pd14.into_output().with_push_pull().build();
    let pin_pwm = gpio.pd13.into_output().with_push_pull().build();
    let timer = p.timer0.into_timer(TimerDivider::Div1024);
    let (tim0ch0, tim0ch1, _tim0ch2, _tim0ch3) = timer.into_channels();

    let mut pwm = tim0ch1.into_pwm(pin_pwm);
    let mut delayer = tim0ch0.into_delay(&clocks);

    println!("{}", &delayer);

    let mut seconds: u32 = 0;
    let mut percent = 0;
    loop {
        if seconds > 10 {
            seconds = 0;
        } else {
            seconds += 2;
        }

        println!("Delay {} seconds, pwm {} %", seconds, percent);

        let _ = pwm.set_duty_cycle_percent(percent);
        percent = if percent < 100 { percent + 10 } else { 0 };

        let _ = pin_delay.toggle();
        delayer.delay_ms(seconds * 1_000);
    }
}
