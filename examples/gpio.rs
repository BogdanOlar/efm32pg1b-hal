//! Build with `cargo build --example gpio`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::{
    gpio::{DataInCtrl, DriveStrengthCtrl},
    prelude::*,
};
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

    gpio.port_f.set_drive_strength(DriveStrengthCtrl::Strong);
    gpio.port_f
        .set_drive_strength_alt(DriveStrengthCtrl::Strong);

    // Don't call `gpio.port_f.set_din_dis(DataInCtrl::Disabled)` because the debug pins are in port `F`
    // But calling `gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled)` is fine since the debug pins use the `Necessary`
    // port `F` ctrl configs
    // TODO: encode this constraint into the type states for the port(s) which contain pins used for SWD/JTAG
    gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled);

    let mut led0 = gpio.pf4.into_output().with_push_pull().build();
    let mut led1 = gpio.pf5.into_output_alt().with_push_pull().build();
    let mut btn0 = gpio.pf6.into_input().build();
    let mut btn1 = gpio.pf7.into_input().build();

    let mut btn0_prev = true;
    let mut btn1_prev = true;

    loop {
        match btn0.is_high() {
            Ok(btn0_cur) => {
                if btn0_prev != btn0_cur {
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
            Err(e) => {
                defmt::println!("btn0: {}", e);
            }
        }

        match btn1.is_high() {
            Ok(btn1_cur) => {
                if btn1_prev != btn1_cur {
                    match led1.toggle() {
                        Ok(_) => {
                            defmt::println!("btn1: {}", &btn1_cur);
                        }
                        Err(e) => {
                            defmt::println!("led1: {}", e);
                        }
                    }
                    btn1_prev = btn1_cur;
                }
            }
            Err(e) => {
                defmt::println!("btn1: {}", e);
            }
        }
    }
}
