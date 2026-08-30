//! Register-level DMA functions

use crate::dma::descriptor::Descriptor;
use crate::dma::{ChReqSel, ChannelId, DmaError, CHANNEL_COUNT};
use crate::pac::Ldma;
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
    dma().chen().read().chen().bits() & (1 << id as u8) != 0
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
    dma().chdone().read().bits() & (1 << id as u8) != 0
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
    dma().chbusy().read().busy().bits() & (1 << id as u8) != 0
}

pub(crate) fn ien(id: ChannelId) -> bool {
    (dma().ien().read().bits() & (1 << id as u8)) != 0
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
    if dma().if_().read().error().bit_is_set() {
        Some(ChannelId::from_u8_unchecked(
            dma().status().read().cherror().bits(),
        ))
    } else {
        None
    }
}

pub(crate) fn if_error_clear() {
    dma().ifc().write(|w| w.error().set_bit());
}

pub(crate) fn swreq(id: ChannelId) {
    dma()
        .swreq()
        .write(|w| unsafe { w.swreq().bits(1 << id as u8) });
}

pub(crate) fn ch_loop(id: ChannelId) -> u8 {
    dma().ch(id as usize).loop_().read().loopcnt().bits()
}

pub(crate) fn ch_loop_set(id: ChannelId, loop_count: u8) {
    dma()
        .ch(id as usize)
        .loop_()
        .write(|w| unsafe { w.loopcnt().bits(loop_count) });
}

/// Set Channel Peripheral Request Select
pub(crate) fn reqsel(id: ChannelId) -> Result<ChReqSel, DmaError> {
    let sig = dma().ch(id as usize).reqsel().read().sigsel().bits();
    let source = dma().ch(id as usize).reqsel().read().sourcesel().bits();
    let raw = ((sig as u16) << 6) | source as u16;

    raw.try_into()
}

/// Set Channel Peripheral Request Select
pub(crate) fn set_reqsel(id: ChannelId, source: ChReqSel) {
    let sig = ((source as u16) & 0b1111) as u8;
    let source = (((source as u16) >> 4) & 0b111111) as u8;

    dma()
        .ch(id as usize)
        .reqsel()
        .write(|w| unsafe { w.sigsel().bits(sig).sourcesel().bits(source) });
}

pub(crate) fn ch_link_load(id: ChannelId) {
    dma()
        .linkload()
        .write(|w| unsafe { w.linkload().bits(1 << id as u8) });
}

pub(crate) fn ch_req_mode_set(id: ChannelId, all: bool) {
    dma()
        .ch(id as usize)
        .ctrl()
        .modify(|_, w| w.reqmode().bit(all));
}

/// WARNING: number of words actually transfered will be `cnt + 1`
pub(crate) fn ch_xfer_cnt_set(id: ChannelId, cnt: u16) {
    dma()
        .ch(id as usize)
        .ctrl()
        .write(|w| unsafe { w.xfercnt().bits(cnt) });
}

pub(crate) fn ch_src_set(id: ChannelId, addr: u32) {
    dma()
        .ch(id as usize)
        .src()
        .write(|w| unsafe { w.srcaddr().bits(addr) });
}

pub(crate) fn ch_dst_set(id: ChannelId, addr: u32) {
    dma()
        .ch(id as usize)
        .dst()
        .write(|w| unsafe { w.dstaddr().bits(addr) });
}

pub(crate) fn ch_write_descriptor(id: ChannelId, descr: &Descriptor) {
    dma()
        .ch(id as usize)
        .ctrl()
        .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_CTRL]) });
    dma()
        .ch(id as usize)
        .src()
        .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_SRC]) });
    dma()
        .ch(id as usize)
        .dst()
        .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_DST]) });
    dma()
        .ch(id as usize)
        .link()
        .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_LINK]) });
}

/// Iterator over all raised channel DMA done flags
pub(crate) fn if_raised() -> impl Iterator<Item = ChannelId> {
    let cached_flags = dma().if_().read().done().bits();

    (0..CHANNEL_COUNT as u8)
        .filter(move |i| ((1 << *i) & cached_flags) != 0)
        .map(ChannelId::from_u8_unchecked)
}

/// Get the DMA (pac) peripheral
pub(crate) fn dma() -> Ldma {
    unsafe { crate::pac::Ldma::steal() }
}
