//! Register-level DMA functions

use crate::dma::descriptor::Descriptor;
use crate::dma::{ChReqSel, ChannelId, DmaError, CHANNEL_COUNT};
use crate::pac::ldma::Ldma;
use crate::SingleCycleRMW;

/// Disable "Synchronization PRS Set Enable"
pub(crate) fn ctrl_syncprsseten_clear(id: ChannelId) {
    dma().ctrl().sc_clear(1 << id as u8);
}

/// Disable "Synchronization PRS Clear Enable"
pub(crate) fn ctrl_syncprsclren_clear(id: ChannelId) {
    dma().ctrl().sc_clear((1 << id as u8) << CHANNEL_COUNT);
}

pub(crate) fn sync_clear(id: ChannelId) {
    dma().sync().sc_clear(1 << id as u8);
}

/// Get channel enabled
pub(crate) fn chen(id: ChannelId) -> bool {
    dma().chen().read().chen() & (1 << id as u8) != 0
}

/// Enable channel
pub(crate) fn chen_set(id: ChannelId) {
    dma().chen().sc_set(1 << id as u8);
}

/// Disable channel
pub(crate) fn chen_clear(id: ChannelId) {
    dma().chen().sc_clear(1 << id as u8);
}

pub(crate) fn ch_done(id: ChannelId) -> bool {
    dma().chdone().read().chdone() & (1 << id as u8) != 0
}

pub(crate) fn ch_done_set(id: ChannelId) {
    dma().chdone().sc_set(1 << id as u8);
}

pub(crate) fn ch_done_clear(id: ChannelId) {
    dma().chdone().sc_clear(1 << id as u8);
}

pub(crate) fn dbghalt_clear(id: ChannelId) {
    dma().dbghalt().sc_clear(1 << id as u8);
}

pub(crate) fn dbghalt_set(id: ChannelId) {
    dma().dbghalt().sc_set(1 << id as u8);
}

pub(crate) fn reqdis_clear(id: ChannelId) {
    dma().reqdis().sc_clear(1 << id as u8);
}

pub(crate) fn reqclear_set(id: ChannelId) {
    dma().reqclear().sc_set(1 << id as u8);
}

pub(crate) fn ch_busy(id: ChannelId) -> bool {
    dma().chbusy().read().busy() & (1 << id as u8) != 0
}

pub(crate) fn ien(id: ChannelId) -> bool {
    (dma().ien().read().done() & (1 << id as u8)) != 0
}

/// Set IEN flag for channel (single-cycle read-modify-write)
pub(crate) fn ien_set(id: ChannelId) {
    dma().ien().sc_set(1 << id as u8);
}

/// Clear IEN flag for channel (single-cycle read-modify-write)
pub(crate) fn ien_clear(id: ChannelId) {
    dma().ien().sc_clear(1 << id as u8);
}

/// Clear interrupt flag for channel (single-cycle read-modify-write)
pub(crate) fn ifc_set(id: ChannelId) {
    dma().ifc().sc_set(1 << id as u8);
}

pub(crate) fn ch_error() -> Option<ChannelId> {
    if dma().if_().read().error() == true {
        Some(ChannelId::from_u8_unchecked(
            dma().status().read().cherror(),
        ))
    } else {
        None
    }
}

pub(crate) fn if_error_clear() {
    dma().ifc().write(|w| w.set_error(true));
}

pub(crate) fn swreq(id: ChannelId) {
    dma()
        .swreq()
        .write(|w| w.set_swreq(1 << id as u8));
}

pub(crate) fn ch_loop(id: ChannelId) -> u8 {
    ch(id).loop_().read().loopcnt()
}

pub(crate) fn ch_loop_set(id: ChannelId, loop_count: u8) {
    ch(id)
        .loop_()
        .write(|w| w.set_loopcnt(loop_count));
}

/// Set Channel Peripheral Request Select
pub(crate) fn reqsel(id: ChannelId) -> Result<ChReqSel, DmaError> {
    let sig = ch(id).reqsel().read().sigsel();
    let source = ch(id).reqsel().read().sourcesel();
    let raw = ((sig as u16) << 6) | source.to_bits() as u16;

    raw.try_into()
}

/// Set Channel Peripheral Request Select
pub(crate) fn set_reqsel(id: ChannelId, source: ChReqSel) {
    let sig = ((source as u16) & 0b1111) as u8;
    let source = (((source as u16) >> 4) & 0b111111) as u8;

    ch(id).reqsel().write(|w| {
        w.set_sigsel(sig);
        w.set_sourcesel(efm32pg1b_pac::ldma::vals::Ch7ReqselSourcesel::from_bits(source));
    });
}

pub(crate) fn ch_link_load(id: ChannelId) {
    dma()
        .linkload()
        .write(|w| w.set_linkload(1 << id as u8));
}

pub(crate) fn ch_req_mode_set(id: ChannelId, all: bool) {
    ch(id)
        .ctrl()
        .modify(|w| w.set_reqmode(all));
}

/// WARNING: number of words actually transfered will be `cnt + 1`
pub(crate) fn ch_xfer_cnt_set(id: ChannelId, cnt: u16) {
    ch(id).ctrl().write(|w| w.set_xfercnt(cnt));
}

pub(crate) fn ch_src_set(id: ChannelId, addr: u32) {
    ch(id).src().write_value(addr);
}

pub(crate) fn ch_dst_set(id: ChannelId, addr: u32) {
    ch(id).dst().write_value(addr);
}

pub(crate) fn ch_write_descriptor(id: ChannelId, descr: &Descriptor) {
    ch(id)
        .ctrl()
        .write_value(efm32pg1b_pac::ldma::regs::Ch7Ctrl(
            descr.raw[Descriptor::INDEX_CTRL],
        ));
    ch(id).src().write_value(descr.raw[Descriptor::INDEX_SRC]);
    ch(id).dst().write_value(descr.raw[Descriptor::INDEX_DST]);
    ch(id)
        .link()
        .write_value(efm32pg1b_pac::ldma::regs::Ch7Link(
            descr.raw[Descriptor::INDEX_LINK],
        ));
}

/// Iterator over all raised channel DMA done flags
pub(crate) fn if_raised() -> impl Iterator<Item = ChannelId> {
    let cached_flags = dma().if_().read().done();

    (0..CHANNEL_COUNT as u8)
        .filter(move |i| ((1 << *i) & cached_flags) != 0)
        .map(ChannelId::from_u8_unchecked)
}

/// Get the DMA (pac) peripheral
pub(crate) fn dma() -> Ldma {
    crate::pac::LDMA
}

/// Get the register block for a given DMA channel.
///
/// The chiptool-generated PAC exposes the LDMA channels as separate `ch0()`..`ch7()` accessors
/// (they are a clustered block), so this helper maps a runtime [`ChannelId`] to the matching
/// channel register block.
pub(crate) fn ch(id: ChannelId) -> efm32pg1b_pac::ldma::Channel {
    let dma = dma();
    match id as u8 {
        0 => dma.ch0(),
        1 => dma.ch1(),
        2 => dma.ch2(),
        3 => dma.ch3(),
        4 => dma.ch4(),
        5 => dma.ch5(),
        6 => dma.ch6(),
        7 => dma.ch7(),
        _ => unreachable!(),
    }
}
