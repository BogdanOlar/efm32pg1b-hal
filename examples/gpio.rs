//! Build with `cargo build --example gpio --features="defmt"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use defmt_rtt as _;
use efm32pg1b_hal::gpio::{
    port::{DataInCtrl, DriveStrength},
    Gpio, InFilt, InFloat, OutPp, OutPpAlt,
};
use efm32pg1b_hal::pac;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use panic_probe as _;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();

    let mut gpio = Gpio::new(p.gpio);

    gpio.port_f.set_drive_strength(DriveStrength::Strong);
    gpio.port_f.set_drive_strength_alt(DriveStrength::Strong);

    // Calling this is fine since the debug pins use the `Primary` not the `Alternate` port `F` ctrl configs
    gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled);

    let mut led0 = gpio.pf4.into_mode::<OutPp>();
    let mut btn0 = gpio.pf6.into_mode::<InFloat>();
    // let mut led1 = gpio.pf5.into_erased_pin().into_mode::<OutPpAlt>();
    // let mut btn1 = gpio.pf7.into_erased_pin().into_mode::<InFilt>();
    let mut led1 = gpio.pf5.into_dynamic_pin().into_mode::<OutPpAlt>();
    let mut btn1 = gpio.pf7.into_dynamic_pin().into_mode::<InFilt>();

    // button states
    let mut btn0_prev = true;
    let mut btn1_prev = true;

    loop {
        // Button 0 and LED 0
        if let Ok(btn0_cur) = btn0.is_high() {
            if btn0_cur != btn0_prev {
                defmt::info!("btn0 {}: {}", &btn0, !btn0_cur);
                led0.toggle().unwrap();
                let ledstate = led0.is_set_high().unwrap();
                defmt::info!("led0 {}: {}", &led0, ledstate);
                btn0_prev = btn0_cur;
            }
        }

        // Button 1 and LED 1
        if let Ok(btn1_cur) = btn1.is_high() {
            if btn1_cur != btn1_prev {
                defmt::info!("btn1 {}: {}", &btn1, !btn1_cur);

                // We can't call `led1.toggle()` since Alternate Data In is Disabled for port F, and the
                // `StatefulOutputPin::toggle()`  will return an `Err` when reading the state of the Pin
                let ledstate = !btn1_cur;
                match ledstate {
                    true => led1.set_high().unwrap(),
                    false => led1.set_low().unwrap(),
                };
                defmt::info!("led1 {}: {}", &led1, ledstate);
                btn1_prev = btn1_cur;
            }
        }
    }
}
