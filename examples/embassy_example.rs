#![no_main]
#![no_std]

use efm32pg1b_hal::{
    cmu::{HfClockPrescaler, HfClockSource, LfClockSource},
    prelude::*,
    timer_le::efemb::Ticker,
};
use embassy_executor::Spawner;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

const TASK_COUNT: usize = 10;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = pac::Peripherals::take().unwrap();
    let clocks = p
        .cmu
        .split()
        // Prescaling the HF clock to the lowest frequency possible, to stress test the scheduler algorithm
        // .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div32)
        // .with_hf_clk(HfClockSource::HfXO(40.MHz()), HfClockPrescaler::Div1);
        .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div1);

    // Make sure LfAClk is enabled otherwise the LeTimer0 Ticker won't work
    #[cfg(feature = "efemb-timdrv-letim0-hz-32_768")]
    let _clocks = clocks.with_lfa_clk(LfClockSource::LfRco);
    #[cfg(feature = "efemb-timdrv-letim0-hz-1_000")]
    let _clocks = clocks.with_lfa_clk(LfClockSource::UlfRco);

    // make sure `defmt` works correctly even if the time driver is not initialized (should show timestamp = 0)
    defmt::info!("\tHello info world!");
    defmt::warn!("\tHello warn world!");
    // Initialize the embassy time driver
    Ticker::init();
    // Timestamps should now be non-zero, monotonically incremented
    defmt::error!("\tHello error world!");
    defmt::trace!("\tHello trace world!");

    for i in 0..TASK_COUNT {
        spawner.spawn(my_task(i).expect("Could not spawn Task"));
    }
}

#[embassy_executor::task(pool_size = TASK_COUNT)]
async fn my_task(task_number: usize) {
    let mut counter: u64 = 0;

    loop {
        Timer::after_millis(1000).await;
        defmt::info!("Task {} ping {}", task_number, counter);
        counter += 1;
    }
}
