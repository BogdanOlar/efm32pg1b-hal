//! Build with `cargo build --example gpio --features="defmt"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

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

    let gpio = p.gpio.split();

    gpio.port_f.set_drive_strength(DriveStrengthCtrl::Strong);
    gpio.port_f
        .set_drive_strength_alt(DriveStrengthCtrl::Strong);

    // This should not be called because the debug pins are `PF0`, `PF1`, `PF2`, `PF3`
    // FIXME: encode this constraint into the type states for the port(s) which contain pins used for SWD/JTAG
    // gpio.port_f.set_din_dis(DataInCtrl::Disabled);

    // FIXME: this should not be permitted because `PF0` is a debug pin
    // let mut dbg_swclk = gpio.pf0.into_disabled();

    // Calling this is fine since the debug pins use the `Primary` not the `Alternate` port `F` ctrl configs
    gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled);

    let mut led0 = gpio.pf4.into_output().with_push_pull().build();
    let mut led1 = gpio.pf5.into_output_alt().with_push_pull().build();
    let mut btn0 = gpio.pf6.into_input().build();
    let mut btn1 = gpio.pf7.into_input().build();

    let mut btn0_prev = true;
    let mut btn1_prev = true;

    loop {
        // Button 0 and LED 0
        if let Ok(btn0_cur) = btn0.is_high() {
            if btn0_cur != btn0_prev {
                match led0.toggle() {
                    Ok(_) => {
                        defmt::println!("led0: {}", &btn0_cur);
                    }
                    Err(e) => {
                        defmt::println!("led0: {}", e);
                    }
                }
                btn0_prev = btn0_cur;
            }
        }

        // Button 1 and LED 1
        if let Ok(btn1_cur) = btn1.is_high() {
            if btn1_cur != btn1_prev {
                // NOTE: Toggle will fail because `led1` was constructed to use ALT port config and `DINDISALT` is
                // asserted. `toggle()` will therefore fail because it's a method of the `StatefulOutputPin` trait,
                // which needs Data In (Alt, in this case) to function correctly.
                match led1.toggle() {
                    Ok(_) => {
                        defmt::println!("btn1: {}", &btn1_cur);
                    }
                    Err(e) => {
                        // will print out "led1 `toggle()`: DataInDisabled"
                        defmt::println!("led1 `toggle()`: {}", e);

                        // NOTE: We can still use the `OutputPin` trait methods, since those don't depent on stateful
                        // output
                        let res = led1.set_state(btn1_prev.into());

                        // will print "led1 `set_state(Low)`: Ok(())" or "led1 `set_state(High)`: Ok(())"
                        defmt::println!("led1 `set_state({})`: {}", PinState::from(btn1_prev), res);
                        // will print out "led1: true" or "led1: false"
                        defmt::println!("btn1: {}", &btn1_cur);
                    }
                }
                btn1_prev = btn1_cur;
            }
        }
    }
}
