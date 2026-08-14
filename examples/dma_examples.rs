//! Implementation for the Reference Manual examples
//!
//! [`7.4.5 Simple Inter-Channel Synchronization`](doc/efm32pg1-rm.pdf#page138)
//!

#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::{error, info};
use defmt_rtt as _;
use efm32pg1b_hal::{
    dma::{
        descriptor::{
            Addr, AddrInc, BlockSize, Descriptor, SyncDescriptor, TransferCount,
            TransferDescriptor, UnitSize,
        },
        list::DescList,
        Dma, DmaChannel, DmaError,
    },
    prelude::*,
    timer_le::efemb::Ticker,
};
use panic_probe as _;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;

const SRC_LEN: usize = 1024 * 10;
static SRC_U8: [u8; SRC_LEN] = {
    let mut seq = [0; SRC_LEN];
    let mut i = 0;
    // Fill the buffer with values from 1 to 254
    while i < SRC_LEN {
        seq[i] = 1 + (i % (u8::MAX - 1) as usize) as u8;
        i += 1;
    }
    seq
};

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    // Initialize the embassy time driver (for defmt timestamps)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    let dma = Dma::init(p.ldma);

    let ret = simple_inter_channel_synchronization(dma.ch0, dma.ch1);
    if let Err(e) = &ret {
        error!("{}", e);
    }
    let (ch0, ch1) = ret.unwrap();

    loop {
        asm::wfe();
    }
}

/// 7.4.5 Simple Inter-Channel Synchronization
///
/// In this example DMA channel 0 and 1 are tasked with the transfer of different sets of data. Channel 0 has two
/// transfer structures, and channel 1 just one, but channel 0 must wait until channel 1 has completed its transfer
/// before it starts its second transfer structure.
///
/// See: doc/efm32pg1-rm.pdf#page138
fn simple_inter_channel_synchronization(
    mut ch0: DmaChannel,
    mut ch1: DmaChannel,
) -> Result<(DmaChannel, DmaChannel), DmaError> {
    info!("Simple Inter-Channel Synchronization");

    ch0.reset();
    ch1.reset();

    // Debug
    ch0.set_dbg_halt();
    ch1.set_dbg_halt();

    let mut ch0_descriptors = [Descriptor::const_default(); 3];
    let mut ch0_desc_list = DescList::new(&mut ch0_descriptors);
    let mut ch1_descriptors = [Descriptor::const_default(); 2];
    let mut ch1_desc_list = DescList::new(&mut ch1_descriptors);

    const TR_SIZE: usize = 10;
    let src_a = &SRC_U8[0..TR_SIZE];
    let mut dst_buf_a = [0u8; TR_SIZE];
    let dst_a = &mut dst_buf_a;
    let src_c = &SRC_U8[TR_SIZE..TR_SIZE * 2];
    let mut dst_buf_c = [0u8; TR_SIZE];
    let dst_c = &mut dst_buf_c;
    let src_y = &SRC_U8[TR_SIZE * 2..TR_SIZE * 3];
    let mut dst_buf_y = [0u8; TR_SIZE];
    let dst_y = &mut dst_buf_y;

    // Descriptor `a`
    ch0_desc_list.push_linked(
        TransferDescriptor::new(
            Addr::Absolute(src_a.as_ptr().addr()),
            Addr::Absolute(dst_a.as_ptr().addr()),
            dst_a.len().try_into()?,
            UnitSize::Byte,
        )
        .with_struct_req(true)
        .with_req_mode_all(true),
    )?;

    // Descriptor `b`
    ch0_desc_list.push_linked(SyncDescriptor::new().with_matchen(0x80).with_matchval(0x80))?;

    // Descriptor `c`
    ch0_desc_list.push_linked(
        TransferDescriptor::new(
            Addr::Absolute(src_c.as_ptr().addr()),
            Addr::Absolute(dst_c.as_ptr().addr()),
            dst_c.len().try_into()?,
            UnitSize::Byte,
        )
        .with_struct_req(true)
        .with_req_mode_all(true),
    )?;

    // Descriptor `y`
    ch1_desc_list.push_linked(
        TransferDescriptor::new(
            Addr::Absolute(src_y.as_ptr().addr()),
            Addr::Absolute(dst_y.as_ptr().addr()),
            dst_y.len().try_into()?,
            UnitSize::Byte,
        )
        .with_struct_req(true)
        .with_req_mode_all(true),
    )?;

    // Descriptor `z`
    ch1_desc_list.push_linked(SyncDescriptor::new().with_syncclr(0x00).with_syncset(0x80))?;

    // Link the descriptor lists for the Channels
    ch0.set_descriptor(ch0_desc_list.try_into_link_descriptor()?);
    ch1.set_descriptor(ch1_desc_list.try_into_link_descriptor()?);

    // Start Channel 0
    ch0.set_enable();
    ch0.set_ien();
    ch0.link_load();

    info!("dst_a: {}", dst_a);
    info!("dst_c: {}", dst_c);
    info!("dst_y: {}", dst_y);

    // At this point Channel 0 has started, descriptor `a` was executed, and the channel is waiting for the Sync coming
    // from Channel 1, which hasn't started yet. This expains why `dst_c` (descriptor `c` on Channel 0) is still `0`:
    //      dst_a: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    //      dst_c: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    //      dst_y: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

    // Start Channel 1
    ch1.set_ien();
    ch1.set_enable();
    ch1.link_load();

    while ch0.busy() || ch1.busy() {
        asm::nop();
    }

    info!("dst_a: {}", dst_a);
    info!("dst_c: {}", dst_c);
    info!("dst_y: {}", dst_y);

    // Channel 1 has started and finished with the sync descriptor, which caused Channel 0 to advance to the `c`
    // descriptor:
    //      dst_a: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    //      dst_c: [11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
    //      dst_y: [21, 22, 23, 24, 25, 26, 27, 28, 29, 30]

    Ok((ch0, ch1))
}
