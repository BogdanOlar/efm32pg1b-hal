#![no_std]
#![no_main]

use defmt::{error, info};
use defmt_rtt as _;
use efm32pg1b_hal::{
    dma::{Dma, DmaChannel},
    pac::{self},
    prelude::*,
    timer_le::efemb::Ticker,
};
use embassy_executor::Spawner;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;
use panic_halt as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = pac::Peripherals::take().unwrap();

    // Initialize the embassy time driver in order to get logging timestamps (LfAClk is ncecessary for LeTimer0)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    // Initialize GPIO
    let dma = Dma::init(p.ldma);

    // ---- Channel 0 ----
    spawner.spawn(transfer(dma.ch0).expect("Could not spawn Task"));

    // ---- Channel 1 ----
    spawner.spawn(transfer(dma.ch1).expect("Could not spawn Task"));

    info!("Starting transfers...");
}

#[embassy_executor::task(pool_size = 2)]
async fn transfer(ch: DmaChannel) {
    const BUF_UNIT_SIZE: usize = 0x400 * 10;

    // const TRANSFER_UNIT_COUNT: usize = 15;
    // const TRANSFER_UNIT_COUNT: usize = 32;
    // const TRANSFER_UNIT_COUNT: usize = 40;
    // const TRANSFER_UNIT_COUNT: usize = 70;
    // const TRANSFER_UNIT_COUNT: usize = 300;
    const TRANSFER_UNIT_COUNT: usize = 0x800 * 4 + 5;

    const SRC_U8: [u8; BUF_UNIT_SIZE] = {
        let mut seq = [0; BUF_UNIT_SIZE];
        let mut i = 0;
        // Fill the buffer with values from 1 to 255
        while i < BUF_UNIT_SIZE {
            seq[i] = 1 + (i % u8::MAX as usize) as u8;
            i += 1;
        }
        seq
    };
    let src = &SRC_U8[0..];
    let mut dst_u8: [u8; BUF_UNIT_SIZE] = [0u8; _];

    // take an unaligned slice of `dst` so that the transfer is done with bytes, not half-words or words
    let dst = &mut dst_u8[1..TRANSFER_UNIT_COUNT + 1];

    // // Half-Word aligned transfer
    // let dst = &mut dst_u8[2..TRANSFER_UNIT_COUNT + 2];

    // // Word-aligned transfer
    // let dst = &mut dst_u8[0..TRANSFER_UNIT_COUNT];

    let transfer_result = ch.into_async_transfer(src, dst).await;

    // Print results
    match &transfer_result {
        Ok((params, bytes_count)) => {
            info!(
                "Ok: {} {} bytes, src[{} bytes @ 0x{:X}], dst[{} bytes @ 0x{:X}]",
                params.ch,
                *bytes_count,
                params.src.len(),
                params.src.as_ptr().addr(),
                params.dst.len(),
                params.dst.as_ptr().addr(),
            );
            if *bytes_count <= 300 {
                info!("dst: {}", params.dst[0..*bytes_count]);
            }
        }
        Err(params) => {
            error!(
                "Err: {}, src[{} bytes @ 0x{:X}], dst[{} bytes @ 0x{:X}]",
                params.ch,
                params.src.len(),
                params.src.as_ptr().addr(),
                params.dst.len(),
                params.dst.as_ptr().addr(),
            );
            if params.dst.len() <= 300 {
                error!("dst: {}", params.dst);
            }
        }
    }
}
