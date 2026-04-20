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

    dma::mmio::init();

    // blocking();
    non_blocking();

    loop {
        asm::wfe();
    }
}

const BUF_UNIT_SIZE: usize = 1024 * 10;
const TRANSFER_UNIT_COUNT: usize = 0x800 * 4 + 5;

fn non_blocking() {
    info!("Non-blocking \n");

    let id = dma::ChannelId::Ch1;

    const SRC_U8: [u8; BUF_UNIT_SIZE] = {
        let mut seq = [0; BUF_UNIT_SIZE];
        let mut i = 0;
        // Fill the buffer with values from 0 to 255
        while i < BUF_UNIT_SIZE {
            seq[i] = 1 + (i % (u8::MAX - 1) as usize) as u8;
            i += 1;
        }
        seq
    };
    let mut dst_u8: [u8; BUF_UNIT_SIZE] = [0u8; _];
    let dst = &mut dst_u8[1..TRANSFER_UNIT_COUNT + 1];
    let res = dma::mmio::transfer_nb(id, &SRC_U8, dst);
    let tr = res.unwrap();

    // const SRC_U16: [u16; BUF_UNIT_SIZE] = {
    //     let mut seq = [0; BUF_UNIT_SIZE];
    //     let mut i = 0;
    //     // Fill the buffer with values from 0 to 255
    //     while i < BUF_UNIT_SIZE {
    //         seq[i] = 1 + (i % (u16::MAX - 1) as usize) as u16;
    //         i += 1;
    //     }
    //     seq
    // };
    // let mut dst_u16: [u16; BUF_UNIT_SIZE] = [0u16; _];
    // let dst = &mut dst_u16[1..TRANSFER_UNIT_COUNT + 1];
    // let res = dma::mmio::transfer_nb(id, &SRC_U16, dst);
    // let tr = res.unwrap();

    while !tr.is_done() {
        // info!(".");
    }

    // let res = tr.resolve();
    let res = tr.resolve();
    info!("Result: {}", res);
    // info!("src: {}", SRC_U16[0..TRANSFER_UNIT_COUNT]);
    // info!("dst: {}", dst);
}

fn blocking() {
    info!("\n Blocking \n");
    let id = dma::ChannelId::Ch0;
    let src: [u8; _] = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut dst: [u8; 10] = [0u8; _];
    let mut total_copied = 0;

    info!("src: {}", src);
    info!("dst: {}", dst);

    let res = dma::mmio::transfer_blocking(id, &src[2..6], &mut dst);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);
    let copied_count = res.unwrap();
    total_copied += copied_count;
    assert_eq!(4, copied_count);
    assert_eq!(dst, [3, 4, 5, 6, 0, 0, 0, 0, 0, 0]);

    let res = dma::mmio::transfer_blocking(id, &src, &mut dst[copied_count..]);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);
    let copied_count = res.unwrap();
    total_copied += copied_count;
    assert_eq!(6, copied_count);
    assert_eq!(dst, [3, 4, 5, 6, 1, 2, 3, 4, 5, 6]);

    // this should "copy" 0 bytes
    let res = dma::mmio::transfer_blocking(id, &src, &mut dst[total_copied..]);
    info!("Result: {}", res);
    info!("src: {}", src);
    info!("dst: {}", dst);
    let copied_count = res.unwrap();
    assert_eq!(0, copied_count);
    assert_eq!(dst, [3, 4, 5, 6, 1, 2, 3, 4, 5, 6]);
}
