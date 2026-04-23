//! Linked Direct Memory Access
//!

use crate::pac::{
    ldma::ch::ctrl::{BLOCKSIZE, DSTINC, SIZE, SRCINC, STRUCTTYPE},
    Ldma,
};
use core::cmp::min;
use cortex_m::asm;

/// DMA driver
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dma {
    pub ch0: DmaChannel,
    pub ch1: DmaChannel,
    pub ch2: DmaChannel,
    pub ch3: DmaChannel,
    pub ch4: DmaChannel,
    pub ch5: DmaChannel,
    pub ch6: DmaChannel,
    pub ch7: DmaChannel,
}

impl Dma {
    pub fn init(_dma_p: Ldma) -> Self {
        // Enable DMA clock
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfbusclken0().modify(|_, w| w.ldma().set_bit());

        Self {
            ch0: DmaChannel { id: ChannelId::Ch0 },
            ch1: DmaChannel { id: ChannelId::Ch1 },
            ch2: DmaChannel { id: ChannelId::Ch2 },
            ch3: DmaChannel { id: ChannelId::Ch3 },
            ch4: DmaChannel { id: ChannelId::Ch4 },
            ch5: DmaChannel { id: ChannelId::Ch5 },
            ch6: DmaChannel { id: ChannelId::Ch6 },
            ch7: DmaChannel { id: ChannelId::Ch7 },
        }
    }
}

/// DMA channel singleton
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DmaChannel {
    /// Channel ID
    id: ChannelId,
}

impl DmaChannel {
    /// Get the DMA channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Start a memory-to-memory transfer
    pub fn try_into_transfer<'a, W: Sized>(
        self,
        src: &'a [W],
        dst: &'a mut [W],
    ) -> Result<ChannelTransfer<'a, W>, DmaError> {
        let total_byte_cnt = min(core::mem::size_of_val(src), core::mem::size_of_val(dst));

        if total_byte_cnt == 0 {
            return Err(DmaError::InvalidTransferSize(self));
        }

        Ok(ChannelTransfer::new(self, src, dst, total_byte_cnt))
    }
}

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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelTransfer<'a, W: Sized> {
    ch: DmaChannel,
    src: &'a [W],
    dst: &'a mut [W],
    byte_count: usize,
}

impl<'a, W: Sized> ChannelTransfer<'a, W> {
    fn new(ch: DmaChannel, src: &'a [W], dst: &'a mut [W], byte_count: usize) -> Self {
        let transfer = Self {
            ch,
            src,
            dst,
            byte_count,
        };

        transfer.start();
        transfer
    }

    /// Start the DMA transfer
    fn start(&self) {
        let src: &[u8] =
            unsafe { core::slice::from_raw_parts(self.src.as_ptr() as *const u8, self.byte_count) };
        let dst: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(self.dst.as_ptr() as *mut u8, self.byte_count)
        };

        assert_eq!(src.len(), dst.len());
        assert_ne!(dst.len(), 0);

        let id = self.ch.id();

        // Decide which unit/type the transfer may use
        let unit = if src.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.len().is_multiple_of(size_of::<u32>())
        {
            SIZE::Word
        } else if src.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.len().is_multiple_of(size_of::<u16>())
        {
            SIZE::Halfword
        } else {
            SIZE::Byte
        };

        let unit_byte_size = 1 << unit as u8;
        let total_units = dst.len() / unit_byte_size;
        assert_eq!(dst.len() % unit_byte_size, 0);

        mmio::ch_disable(id);
        mmio::if_clear(id);

        let arr_end = dst[dst.len()..].as_ptr().addr();
        let aligned_end_addr = arr_end - (arr_end % align_of::<SerializedDescriptor>());

        let last_descr_addr = aligned_end_addr - size_of::<SerializedDescriptor>();
        let last_chunk_min_units = (arr_end - last_descr_addr) / unit_byte_size;
        assert_eq!((arr_end - last_descr_addr) % unit_byte_size, 0);

        let descr_count = if total_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
            total_units / Descriptor::MAX_TRANSFER_UNITS
        } else {
            (total_units / Descriptor::MAX_TRANSFER_UNITS) + 1
        };
        // First descriptor will be written to DMA channel register, not to the descriptor list
        let linked_list_count = descr_count - 1;
        let linked_list_start_addr =
            aligned_end_addr - (linked_list_count * size_of::<SerializedDescriptor>());
        // Create the descriptor list at the end of the destination buffer
        let descriptor_list = unsafe {
            core::slice::from_raw_parts_mut(
                linked_list_start_addr as *mut SerializedDescriptor,
                linked_list_count,
            )
        };

        let mut remaining_units = total_units;

        // Create first descriptor
        let first_descr_units = if remaining_units > Descriptor::MAX_TRANSFER_UNITS {
            min(
                Descriptor::MAX_TRANSFER_UNITS,
                remaining_units - last_chunk_min_units,
            )
        } else {
            remaining_units
        };
        remaining_units -= first_descr_units;
        let first_descriptor = Descriptor {
            ctrl: Ctrl::new()
                .with_size(unit)
                .with_struct_req()
                .with_block_size(BLOCKSIZE::All)
                .with_xfer_cnt(first_descr_units.try_into().unwrap()),
            src: src.as_ptr().addr(),
            dst: dst.as_ptr().addr(),
            link: if remaining_units > 0 {
                Link::new().with_absolute_addr(descriptor_list.as_ptr().addr())
            } else {
                Link::default()
            },
        };

        // Fill in the linked descriptors
        for (i, ser_descr) in descriptor_list.iter_mut().enumerate() {
            let is_last = i == (linked_list_count - 1);

            let descr_units = if is_last {
                remaining_units
            } else {
                min(
                    Descriptor::MAX_TRANSFER_UNITS,
                    remaining_units - last_chunk_min_units,
                )
            };
            assert!(descr_units <= Descriptor::MAX_TRANSFER_UNITS);

            let addr_offset = (total_units - remaining_units) * unit_byte_size;

            let descr = Descriptor {
                ctrl: Ctrl::new()
                    .with_size(unit)
                    .with_struct_req()
                    .with_block_size(BLOCKSIZE::All)
                    .with_xfer_cnt(descr_units.try_into().unwrap()),
                src: src.as_ptr().addr() + addr_offset,
                dst: dst.as_ptr().addr() + addr_offset,
                link: if !is_last {
                    Link::new().with_relative_addr(1)
                } else {
                    Link::default()
                },
            };

            *ser_descr = descr.into();

            remaining_units -= descr_units;
        }
        assert_eq!(remaining_units, 0);

        // make sure aly linked descriptors have been written before proceeding
        asm::dsb();

        // First descriptor is always written directly to the DMA peripheral in order to support transfers smaller than
        // the size of a descriptor
        mmio::ch_write_descriptor(id, &first_descriptor);

        // start the transfer
        mmio::ch_done_clear(id);
        mmio::ch_enable(id);
        mmio::ch_start(id);
        // ch_link_load(id);
    }

    /// Check if DMA transfer is done
    pub fn is_done(&self) -> bool {
        mmio::ch_done(self.ch.id()) || mmio::ch_error(self.ch.id())
    }

    /// FIXME: need to refactor this because it has some problems
    /// - can ba called twice --> BAD
    /// - can be called before transfer is complete, in which case the channel may be dropped before the DMA has
    ///   finished the transfer --> **VERY BAD** because we may write over valid stack frames, for example.
    pub fn resolve(self) -> Result<(DmaChannel, usize), DmaChannel> {
        if mmio::ch_error(self.ch.id()) {
            Err(self.ch)
        } else if mmio::ch_done(self.ch.id()) {
            Ok((self.ch, self.byte_count))
        } else {
            Err(self.ch)
        }
    }
}

/// DMA Error
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DmaError {
    /// Invalid transfer size (e.g. transfer size may not be 0)
    InvalidTransferSize(DmaChannel),
}

/// DMA Channel Descriptor
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct Descriptor {
    ctrl: Ctrl,
    src: usize,
    dst: usize,
    link: Link,
}

impl Descriptor {
    /// Maximum number of units (byte, half-word, word) which can be transfered in one DMA shot
    const MAX_TRANSFER_UNITS: usize = 1 << 12;
    // /// FIXME: remove this before merge
    // const MAX_TRANSFER_UNITS: usize = 1 << 5;
}

impl From<Descriptor> for SerializedDescriptor {
    fn from(value: Descriptor) -> Self {
        Self {
            raw: [
                value.ctrl.into(),
                value.src as u32,
                value.dst as u32,
                value.link.into(),
            ],
        }
    }
}

/// Serialized DMA Descriptor
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct SerializedDescriptor {
    raw: [u32; 4],
}

/// CTRL register
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Ctrl {
    /// [31]
    dst_mode: AddrMode,
    /// [30]
    src_mode: AddrMode,
    /// [29:28]
    dst_inc: DSTINC,
    /// [27:26]
    size: SIZE,
    /// [25:24]
    src_inc: SRCINC,
    /// [23]
    ignore_s_req: bool,
    /// [22]
    dec_loop_cnt: bool,
    /// [21]
    req_mode: bool,
    /// Setting this bit will set the interrupt flag when the transfer is done, or linked in the case where the LINK
    /// bit is set, or synchronized in the case of a SYNC transfer.
    ///
    /// [20]
    done_if_s_en: bool,
    /// [19:16]
    block_size: BLOCKSIZE,
    /// [15]
    byte_swap: bool,
    /// The `XFERCNT` field specifies number of unit data (words, half-words, or bytes) to transfer, as determined
    /// by the SIZE field.
    /// **The value written should be one less than the desired transfer count**
    ///
    /// [14:4]
    xfer_cnt: u16,
    /// [3]
    struct_req: bool,
    /// [1:0]
    struct_type: STRUCTTYPE,
}

impl Ctrl {
    fn new() -> Self {
        Self::default()
    }

    fn with_size(mut self, unit: SIZE) -> Self {
        self.size = unit;
        self
    }

    fn with_struct_req(mut self) -> Self {
        self.struct_req = true;
        self
    }

    fn with_block_size(mut self, blocksize: BLOCKSIZE) -> Self {
        self.block_size = blocksize;
        self
    }

    /// Set the transfer count. Must be `<= Descriptor::MAX_TRANSFER_UNITS` (`0x800` units)
    ///
    /// Example:
    ///
    /// ```rust,no_run
    ///     Ctrl::new().with_xfer_cnt(0x800usize.try_into().unwrap())
    /// ```
    fn with_xfer_cnt(mut self, units: TransferCount) -> Ctrl {
        self.xfer_cnt = units.count - 1;
        self
    }
}

impl Default for Ctrl {
    fn default() -> Self {
        Self {
            struct_type: STRUCTTYPE::Transfer,
            struct_req: Default::default(),
            xfer_cnt: Default::default(),
            byte_swap: Default::default(),
            block_size: BLOCKSIZE::Unit1,
            done_if_s_en: Default::default(),
            req_mode: Default::default(),
            dec_loop_cnt: Default::default(),
            ignore_s_req: Default::default(),
            src_inc: SRCINC::One,
            size: SIZE::Byte,
            dst_inc: DSTINC::One,
            src_mode: Default::default(),
            dst_mode: Default::default(),
        }
    }
}

impl From<Ctrl> for u32 {
    fn from(value: Ctrl) -> Self {
        let mut ret = 0;
        ret |= (value.dst_mode as u32) << 31;
        ret |= (value.src_mode as u32) << 30;
        ret |= (value.dst_inc as u32) << 28;
        ret |= (value.size as u32) << 26;
        ret |= (value.src_inc as u32) << 24;
        ret |= (value.ignore_s_req as u32) << 23;
        ret |= (value.dec_loop_cnt as u32) << 22;
        ret |= (value.req_mode as u32) << 21;
        ret |= (value.done_if_s_en as u32) << 20;
        ret |= (value.block_size as u32) << 16;
        ret |= (value.byte_swap as u32) << 15;
        ret |= ((value.xfer_cnt as u32) & 0x3FFFFFFF) << 4;
        ret |= (value.struct_req as u32) << 3;
        ret |= value.struct_type as u32;
        ret
    }
}

/// Wrapper for the CTRL.XFERCNT which makes sure the value is at most `Descriptor::MAX_TRANSFER_UNITS` (`0x800`)
struct TransferCount {
    count: u16,
}

impl TryFrom<u32> for TransferCount {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        (value as usize).try_into()
    }
}

impl TryFrom<usize> for TransferCount {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value <= Descriptor::MAX_TRANSFER_UNITS {
            Ok(Self {
                count: value as u16,
            })
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Link {
    /// Link Structure Address
    ///
    /// **WARNING:** the value of this field needs to be expressed in 32-bit words, not in bytes
    ///
    /// - For `Absolute` addressing, right-shift the byte addres by 2. E.g. if the descriptor is at `0x200056F4`,
    ///   then this field needs to contain `0x200056F4 >> 2`, which is `0x80015BD`
    ///
    /// - For `Relative` addressing, if you need to point to the next descriptor in memory, right-shift the size of
    ///   the descriptor by 2, so write `4` to point to the next descriptor, `8` to jump to the one after that, etc.
    ///
    /// [31:2]
    link_addr: usize,
    /// [1]
    link: bool,
    /// [0]
    link_mode: AddrMode,
}

impl Link {
    fn new() -> Self {
        Self::default()
    }

    fn with_absolute_addr(mut self, addr: usize) -> Link {
        self.link_addr = addr >> 2;
        self.link = true;
        self.link_mode = AddrMode::Absolute;
        self
    }

    fn with_relative_addr(mut self, count: isize) -> Link {
        self.link_addr = (count * size_of::<SerializedDescriptor>() as isize) as usize >> 2;
        self.link = true;
        self.link_mode = AddrMode::Relative;
        self
    }
}

impl From<Link> for u32 {
    fn from(value: Link) -> Self {
        let mut ret = 0;
        ret |= (value.link_addr as u32) << 2;
        ret |= (value.link as u32) << 1;
        ret |= value.link_mode as u32;
        ret
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum AddrMode {
    #[default]
    Absolute = 0,
    Relative = 1,
}

/// Register-level DMA functions
pub(crate) mod mmio {
    use crate::dma::{ChannelId, Descriptor};
    use crate::pac::Ldma;

    /// Enable channel
    pub(crate) fn ch_enable(id: ChannelId) {
        dma().chen().sc_set(1 << id as u8);
    }

    /// Disable channel
    pub(crate) fn ch_disable(id: ChannelId) {
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

    pub(crate) fn ch_error(id: ChannelId) -> bool {
        dma().status().read().cherror().bits() == id as u8
    }

    pub(crate) fn ch_busy(id: ChannelId) -> bool {
        dma().chbusy().read().busy().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn ien(id: ChannelId) {
        dma()
            .ien()
            .modify(|r, w| unsafe { w.done().bits(r.done().bits() | (1 << id as u8)) });
    }

    pub(crate) fn if_clear(id: ChannelId) {
        dma()
            .ifc()
            .write(|w| unsafe { w.done().bits(1 << id as u8) });
    }

    pub(crate) fn ch_start(id: ChannelId) {
        dma()
            .swreq()
            .write(|w| unsafe { w.swreq().bits(1 << id as u8) });
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
            .write(|w| unsafe { w.bits(descr.ctrl.into()) });
        dma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.bits(descr.src as u32) });
        dma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.bits(descr.dst as u32) });
        dma()
            .ch(id as usize)
            .link()
            .write(|w| unsafe { w.bits(descr.link.into()) });
    }

    /// Peripheral single-cycle read-modify-write
    ///
    /// The EFM32 Gecko supports bit set and bit clear access to all peripherals except those listed in
    /// Table 4.1 Peripherals that Do Not Support Bit Set and Bit Clear on page 38. The bit set and bit clear functionality
    /// (also called Bit Access) enables modification of bit fields (single bit or multiple bit wide) without the need to
    /// perform a read-modify-write (though it is functionally equivalent). Also, the operation is contained within a single
    /// bus access (for HF peripherals), unlike the Bit-banding operation described in section 4.2.2 Bit-banding which
    /// consumes two bus accesses per operation. All AHB masters can utilize this feature.
    ///
    /// See [Documentation](../../doc/efm32pg1-rm.pdf#page919)
    ///
    /// FIXME: don't implement this for EMU, RMU, and CRYOTIMER perypherals!
    trait SingleCycleRMW {
        /// Single cycle bit(s) set
        fn sc_set(&self, mask: u32);
        /// Single cycle bit(s) clear
        fn sc_clear(&self, mask: u32);
    }

    impl<R> SingleCycleRMW for crate::pac::generic::Reg<R>
    where
        R: crate::pac::generic::RegisterSpec,
    {
        fn sc_set(&self, mask: u32) {
            const BIT_SET_BASE_ADDR: usize = 0x46000000;
            const PERIPHERALS_BASE_ADDR: usize = 0x40000000;

            let addr = BIT_SET_BASE_ADDR + (self.as_ptr().addr() - PERIPHERALS_BASE_ADDR);

            unsafe { (addr as *mut u32).write_volatile(mask) };
        }

        fn sc_clear(&self, mask: u32) {
            const BIT_CLEAR_BASE_ADDR: usize = 0x44000000;
            const PERIPHERALS_BASE_ADDR: usize = 0x40000000;

            let addr = BIT_CLEAR_BASE_ADDR + (self.as_ptr().addr() - PERIPHERALS_BASE_ADDR);

            unsafe { (addr as *mut u32).write_volatile(mask) };
        }
    }

    /// Get the DMA (pac) peripheral
    fn dma() -> Ldma {
        unsafe { crate::pac::Ldma::steal() }
    }
}
