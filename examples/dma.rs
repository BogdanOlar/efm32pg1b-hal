#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use efm32pg1b_hal::{
    dma,
    pac::{Interrupt, NVIC},
    prelude::*,
    timer_le::efemb::Ticker,
};
use panic_probe as _;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;

#[entry]
fn main() -> ! {
    let mut core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();

    // ---- NVIC ----
    unsafe {
        NVIC::unmask(Interrupt::GPIO_EVEN);
        NVIC::unmask(Interrupt::GPIO_ODD);
    }

    // Initialize the embassy time driver (for defmt timestamps)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    let id = dma::ChannelId::Ch0;
    let src: [u8; _] = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut dst: [u8; 10] = [0u8; _];

    info!("src: {}", src);
    info!("dst: {}", dst);

    dma::mmio::init();
    let res = dma::mmio::ch_transfer_blocking(id, &src[2..6], &mut dst[0..10]);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);

    let copied_count = res.unwrap();

    let res = dma::mmio::ch_transfer_blocking(id, &src[0..], &mut dst[copied_count..]);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);

    loop {
        asm::wfe();
    }
}
