#![no_std]
#![no_main]

use defmt_rtt as _;
use efm32pg1b_hal::{
    gpio::{dynamic::DynamicPin, efemb::AsyncInputPin},
    pac::{self, Interrupt, NVIC},
    prelude::*,
    timer_le::efemb::Ticker,
};
use embassy_executor::Spawner;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;
use embedded_hal_async::digital::Wait;
use panic_halt as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = pac::Peripherals::take().unwrap();
    let _clocks = p
        .cmu
        .split()
        .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div1)
        .with_lfa_clk(LfClockSource::LfRco);

    // Initialize the embassy time driver
    Ticker::init();

    let gpio = Gpio::new(p.gpio);

    // ---- NVIC ----
    unsafe {
        NVIC::unmask(Interrupt::GPIO_EVEN);
        NVIC::unmask(Interrupt::GPIO_ODD);
    }

    defmt::info!("press BTN0 (PF6) or BTN1 (PF7)");

    // ---- Button 0 ----
    let led0 = gpio.pf4.into_mode::<OutPp>().into_dynamic_pin();
    let btn0 = gpio
        .pf6
        .into_mode::<InFloat>()
        .into_async_input(gpio.exti4ctrl);
    spawner.spawn(button_task(btn0, led0).expect("Could not spawn Task"));

    // ---- Button 1 ----
    let led1 = gpio.pf5.into_mode::<OutPpAlt>().into_dynamic_pin();
    let btn1 = gpio
        .pf7
        .into_mode::<InFilt>()
        .into_dynamic_pin()
        .try_into_async_input(gpio.exti5ctrl)
        .unwrap();
    spawner.spawn(button_task(btn1, led1).expect("Could not spawn Task"));
}

#[embassy_executor::task(pool_size = 2)]
async fn button_task(mut btn: AsyncInputPin, mut led: DynamicPin) {
    loop {
        // Wait for button press (active low)
        let _ = btn.wait_for_low().await;
        defmt::info!("{} pressed", &btn);

        // Toggle LED
        let _ = led.toggle();

        // Wait for button release
        let _ = btn.wait_for_high().await;
        defmt::info!("{} released", &btn);
    }
}
