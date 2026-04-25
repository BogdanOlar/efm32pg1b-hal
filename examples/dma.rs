#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use efm32pg1b_hal::{
    dma::{Dma, DmaChannel},
    prelude::*,
    timer_le::efemb::Ticker,
};
use panic_probe as _;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;

const BUF_UNIT_SIZE: usize = 1024 * 10;

// const TRANSFER_UNIT_COUNT: usize = 15;
// const TRANSFER_UNIT_COUNT: usize = 32;
// const TRANSFER_UNIT_COUNT: usize = 40;
// const TRANSFER_UNIT_COUNT: usize = 70;
const TRANSFER_UNIT_COUNT: usize = 300;
// const TRANSFER_UNIT_COUNT: usize = 0x800 * 4 + 5;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let dma = Dma::init(p.ldma);

    // Initialize the embassy time driver (for defmt timestamps)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    transfer(dma.ch1);

    // DEBUG: make sure we did not drop the transfer while the DMA channel was not done
    {
        let p = unsafe { pac::Peripherals::steal() };
        let dma = Dma::init(p.ldma);
        assert!(!dma.ch1.busy());
    }

    info!("Done.");

    loop {
        asm::wfe();
    }
}

fn transfer(ch: DmaChannel) {
    info!("Transfer");

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

    info!("src: {} bytes @ 0x{:X}", src.len(), src.as_ptr().addr());
    info!("dst: {} bytes @ 0x{:X}", dst.len(), dst.as_ptr().addr());

    let transfer = ch.try_into_transfer(&SRC_U8, dst).unwrap();

    info!("Using <{}> size unit transfer", transfer.unit());

    let result_token = loop {
        match transfer.check_done() {
            Some(token) => break token,
            None => {
                info!(".")
            }
        }
    };

    let res = transfer.resolve(result_token);
    info!("Result: {}", res);

    info!("dst: {}", dst);
}
