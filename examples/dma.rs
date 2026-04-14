#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use efm32pg1b_hal::{dma, prelude::*, timer_le::efemb::Ticker};
use panic_probe as _;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();

    // Initialize the embassy time driver (for defmt timestamps)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    let id = dma::ChannelId::Ch0;
    let src: [u8; _] = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut dst: [u8; 10] = [0u8; _];
    let mut total_copied = 0;

    info!("src: {}", src);
    info!("dst: {}", dst);

    dma::mmio::init();

    let res = dma::mmio::ch_transfer_blocking(id, &src[2..6], &mut dst);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);
    let copied_count = res.unwrap();
    total_copied += copied_count;
    assert_eq!(4, copied_count);
    assert_eq!(dst, [3, 4, 5, 6, 0, 0, 0, 0, 0, 0]);

    let res = dma::mmio::ch_transfer_blocking(id, &src, &mut dst[copied_count..]);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);
    let copied_count = res.unwrap();
    total_copied += copied_count;
    assert_eq!(6, copied_count);
    assert_eq!(dst, [3, 4, 5, 6, 1, 2, 3, 4, 5, 6]);

    // this should "copy" 0 bytes
    let res = dma::mmio::ch_transfer_blocking(id, &src, &mut dst[total_copied..]);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);
    let copied_count = res.unwrap();
    assert_eq!(0, copied_count);
    assert_eq!(dst, [3, 4, 5, 6, 1, 2, 3, 4, 5, 6]);

    loop {
        asm::wfe();
    }
}
