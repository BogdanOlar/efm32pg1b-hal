#![no_std]
#![no_main]

use cortex_m_rt::entry;
use efm32pg1b_hal::{pac::Peripherals, prelude::*};
use panic_abort as _;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let mut cmu = peripherals.CMU.freeze();
    let gpio = peripherals.GPIO.split(&mut cmu);

    let mut led0 = gpio.pf4.push_pull_output(false);
    let mut led1 = gpio.pf5.push_pull_output(false);

    // External pull-up resistor is too weak. Touching the backside of the
    // board makes the input toggle. Enable the internal pull-up improve
    // input noise resistance.
    let btn0 = gpio.pf6.pull_up().input();
    let btn1 = gpio.pf7.pull_up().input();

    // Each button controls a LED.
    loop {
        if btn0.is_low().unwrap() {
            led0.set_high().ok();
        } else {
            led0.set_low().ok();
        }

        if btn1.is_low().unwrap() {
            led1.set_high().ok();
        } else {
            led1.set_low().ok();
        }
    }
}
