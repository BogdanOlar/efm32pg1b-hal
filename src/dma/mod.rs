//! Linked Direct Memory Access
//!

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChannelId {
    /// Channel 0
    Ch0,
    /// Channel 1
    Ch1,
    /// Channel 2
    Ch2,
    /// Channel 3
    Ch3,
    /// Channel 4
    Ch4,
    /// Channel 5
    Ch5,
    /// Channel 6
    Ch6,
    /// Channel 7
    Ch7,
}

pub mod mmio {
    use crate::dma::ChannelId;
    use crate::pac::Ldma;
    use core::cmp::min;
    use cortex_m::asm;

    pub fn init() {
        let cmu = unsafe { crate::pac::Cmu::steal() };

        // Enable DMA
        cmu.hfbusclken0().modify(|_, w| w.ldma().set_bit());
    }

    pub fn ch_enable(id: ChannelId) {
        ldma()
            .chen()
            .modify(|_, w| unsafe { w.bits(1 << id as u8) });
    }

    pub(crate) fn if_clear(id: ChannelId) {
        ldma().ch(id as usize).cfg().write(|w| unsafe { w.bits(0) });
    }

    pub fn ch_done(id: ChannelId) -> bool {
        ldma().chdone().read().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn ch_busy(id: ChannelId) -> bool {
        ldma().chbusy().read().busy().bits() & (1 << id as u8) != 0
    }

    pub fn ch_start(id: ChannelId) {
        ldma()
            .swreq()
            .write(|w| unsafe { w.swreq().bits(1 << id as u8) });
    }

    pub fn ch_transfer_blocking(id: ChannelId, src: &[u8], dst: &mut [u8]) -> Result<usize, ()> {
        let copy_count = min(src.len(), dst.len());
        let copy_count = min(copy_count, 0b11_1111_1111);

        if copy_count > 0 {
            ch_src_set(id, src);
            ch_dst_set(id, dst);
            ch_xfer_cnt_set(id, copy_count as u16 - 1);
            ch_req_mode_set(id, true);
            ch_enable(id);
            ch_start(id);

            while !ch_done(id) {
                asm::nop();
            }
        }

        Ok(copy_count)
    }

    pub(crate) fn ch_req_mode_set(id: ChannelId, all: bool) {
        ldma()
            .ch(id as usize)
            .ctrl()
            .modify(|_, w| w.reqmode().bit(all));
    }

    /// WARNING: number of words actually transfered will be `cnt + 1`
    pub(crate) fn ch_xfer_cnt_set(id: ChannelId, cnt: u16) {
        ldma()
            .ch(id as usize)
            .ctrl()
            .write(|w| unsafe { w.xfercnt().bits(cnt) });
    }

    pub(crate) fn ch_src_set(id: ChannelId, src: &[u8]) {
        let addr: u32 = src.as_ptr().addr() as u32;
        ldma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.srcaddr().bits(addr) });
    }

    pub(crate) fn ch_dst_set(id: ChannelId, dst: &mut [u8]) {
        let addr: u32 = dst.as_ptr().addr() as u32;
        ldma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.dstaddr().bits(addr) });
    }

    /// Get the DMA (pac) peripheral
    fn ldma() -> Ldma {
        unsafe { crate::pac::Ldma::steal() }
    }
}
