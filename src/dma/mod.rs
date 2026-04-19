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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelTransfer<'a, W: Sized> {
    id: ChannelId,
    src: &'a [W],
    dst: &'a mut [W],
    count: usize,
}

impl<'a, W: Sized> ChannelTransfer<'a, W> {
    pub fn is_done(&self) -> bool {
        mmio::ch_done(self.id) || mmio::ch_error(self.id)
    }

    pub fn resolve(self) -> Result<(ChannelId, usize), ChannelId> {
        if mmio::ch_error(self.id) {
            Err(self.id)
        } else if mmio::ch_done(self.id) {
            Ok((self.id, self.count))
        } else {
            Err(self.id)
        }
    }
}

/// DMA Error
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DmaError {
    /// Invalid transfer size (e.g. transfer size may not be 0)
    InvalidTransferSize(ChannelId),
}

pub mod mmio {
    use crate::dma::{ChannelId, ChannelTransfer, DmaError};
    use crate::pac::{
        ldma::ch::ctrl::{BLOCKSIZE, DSTINC, SIZE, SRCINC, STRUCTTYPE},
        Ldma,
    };
    use core::cmp::min;
    use cortex_m::asm;
    use defmt::info;

    /// DMA Channel Descriptor
    #[derive(Clone, Copy, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    struct Descriptor {
        ctrl: Ctrl,
        src: usize,
        dst: usize,
        link: Link,
    }

    impl Descriptor {
        /// Maximum number of units (byte, half-word, word) which can be transfered in one DMA shot
        /// (i.e. without the need for a linked descriptor list)
        const MAX_TRANSFER_UNITS: usize = 1 << 12;

        /// [`Ctrl::xfer_cnt`] bit mask
        const CTRL_XFER_MASK: usize = Self::MAX_TRANSFER_UNITS - 1;
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
            ret |= ((value.xfer_cnt as u32) & Descriptor::CTRL_XFER_MASK as u32) << 4;
            ret |= (value.struct_req as u32) << 3;
            ret |= value.struct_type as u32;
            ret
        }
    }

    #[derive(Clone, Copy, Debug, Default)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    struct Link {
        /// [31:2]
        link_addr: usize,
        /// [1]
        link: bool,
        /// [0]
        link_mode: AddrMode,
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

    pub fn init() {
        let cmu = unsafe { crate::pac::Cmu::steal() };

        // Enable DMA
        cmu.hfbusclken0().modify(|_, w| w.ldma().set_bit());
    }

    pub fn transfer_blocking(id: ChannelId, src: &[u8], dst: &mut [u8]) -> Result<usize, DmaError> {
        let copy_count = min(src.len(), dst.len());
        let copy_count = min(copy_count, Descriptor::MAX_TRANSFER_UNITS);

        if copy_count > 0 {
            ch_src_set(id, src.as_ptr().addr() as u32);
            ch_dst_set(id, dst.as_ptr().addr() as u32);
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

    pub fn transfer_nb<'a, W: Sized>(
        id: ChannelId,
        src: &'a [W],
        dst: &'a mut [W],
    ) -> Result<ChannelTransfer<'a, W>, DmaError> {
        let total_byte_cnt = min(core::mem::size_of_val(src), core::mem::size_of_val(dst));

        if total_byte_cnt == 0 {
            return Err(DmaError::InvalidTransferSize(id));
        }

        process_transfer(
            id,
            unsafe { core::slice::from_raw_parts(src.as_ptr() as *const u8, total_byte_cnt) },
            unsafe { core::slice::from_raw_parts_mut(dst.as_ptr() as *mut u8, total_byte_cnt) },
        );

        Ok(ChannelTransfer {
            id,
            src,
            dst,
            count: total_byte_cnt,
        })
    }

    fn process_transfer(id: ChannelId, src: &[u8], dst: &mut [u8]) {
        assert_eq!(src.len(), dst.len());
        assert_ne!(dst.len(), 0);

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

        // DEBUG:
        {
            let src_start = src.as_ptr().addr();
            let src_end = src[src.len()..].as_ptr().addr();
            let dst_start = dst.as_ptr().addr();
            let dst_end = dst[dst.len()..].as_ptr().addr();
            info!("SRC: [0x{:X}, 0x{:X})", src_start, src_end);
            info!("DST: [0x{:X}, 0x{:X})", dst_start, dst_end);
        }

        // First descriptor is always written directly to the DMA peripheral in order to support transfers smaller than
        // the size of a descriptor
        let first_desc = if total_units <= Descriptor::MAX_TRANSFER_UNITS {
            // this is a one-chunk transfer, so no linked list necessary
            let fd = Descriptor {
                ctrl: Ctrl {
                    size: unit,
                    req_mode: true,
                    done_if_s_en: true,
                    block_size: BLOCKSIZE::All,
                    xfer_cnt: total_units as u16 - 1,
                    ..Default::default()
                },
                src: src.as_ptr().addr(),
                dst: dst.as_ptr().addr(),
                link: Link {
                    link: false,
                    ..Default::default()
                },
            };

            info!("First descriptor:");
            print_desc(&fd);

            fd
        } else {
            // this is a multi-chunk transfer, so we need a linked list in addition to the first chunk descriptor

            let arr_end = dst[dst.len()..].as_ptr().addr();
            let aligned_end_addr = arr_end - (arr_end % align_of::<SerializedDescriptor>());

            let last_descr_addr = aligned_end_addr - size_of::<SerializedDescriptor>();
            let last_chunk_min_units = (arr_end - last_descr_addr) / unit_byte_size;
            assert_eq!((arr_end - last_descr_addr) % unit_byte_size, 0);

            // Create the descriptor list at the end of the destination buffer
            let descr_count = if total_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
                total_units / Descriptor::MAX_TRANSFER_UNITS
            } else {
                (total_units / Descriptor::MAX_TRANSFER_UNITS) + 1
            };
            // First descriptor will be written to DMA channel register, not to the descriptor list
            let linked_list_size = descr_count - 1;
            let linked_list_start_addr =
                aligned_end_addr - (linked_list_size * size_of::<SerializedDescriptor>());
            let descriptor_list = unsafe {
                core::slice::from_raw_parts_mut(
                    linked_list_start_addr as *mut SerializedDescriptor,
                    linked_list_size,
                )
            };

            // DEBUG:
            {
                info!(
                    "Linked list items: {}, starting at 0x{:X}",
                    linked_list_size,
                    descriptor_list.as_ptr().addr()
                );
            }

            let mut remaining_units = total_units;

            // Create first descriptor
            let first_descr_units = min(
                Descriptor::MAX_TRANSFER_UNITS,
                remaining_units - last_chunk_min_units,
            );
            remaining_units -= first_descr_units;
            let fd = Descriptor {
                ctrl: Ctrl {
                    size: unit,
                    req_mode: true,
                    done_if_s_en: false,
                    block_size: BLOCKSIZE::All,
                    xfer_cnt: first_descr_units as u16 - 1,
                    ..Default::default()
                },
                src: src.as_ptr().addr(),
                dst: dst.as_ptr().addr(),
                link: Link {
                    link: true,
                    link_addr: descriptor_list.as_ptr().addr(),
                    link_mode: AddrMode::Absolute,
                },
            };
            info!("First descriptor:");
            print_desc(&fd);

            // Fill in the linked descriptors
            for (i, ser_descr) in descriptor_list.iter_mut().enumerate() {
                let is_last = i == (linked_list_size - 1);
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
                    ctrl: Ctrl {
                        size: unit,
                        req_mode: true,
                        done_if_s_en: is_last,
                        block_size: BLOCKSIZE::All,
                        xfer_cnt: descr_units as u16 - 1,
                        ..Default::default()
                    },
                    src: src.as_ptr().addr() + addr_offset,
                    dst: dst.as_ptr().addr() + addr_offset,
                    link: Link {
                        link_addr: size_of::<SerializedDescriptor>(),
                        link: !is_last,
                        link_mode: AddrMode::Relative,
                    },
                };
                info!("Linked descriptor [{}]", i);
                print_desc(&descr);

                *ser_descr = descr.into();

                remaining_units -= descr_units;
            }
            assert_eq!(remaining_units, 0);

            // make sure descriptors have been written before proceeding
            asm::dsb();

            // return the first descriptor
            fd
        };

        // Write the first descriptor to the DMA peripheral channel descriptor registers
        dma()
            .ch(id as usize)
            .ctrl()
            .write(|w| unsafe { w.bits(first_desc.ctrl.into()) });
        dma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.bits(first_desc.src as u32) });
        dma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.bits(first_desc.dst as u32) });
        dma()
            .ch(id as usize)
            .link()
            .write(|w| unsafe { w.bits(first_desc.link.into()) });

        // start the transfer
        ch_enable(id);
        ch_start(id);
        // ch_link_load(id);
    }

    /// Debug function to pretty-print a descriptor
    fn print_desc(desc: &Descriptor) {
        info!("\nDescriptor:\n\tCtrl:\n\t\tdst_mode: {}\n\t\tsrc_mode: {}\n\t\tdst_inc: {}\n\t\tsize: {}\n\t\tsrc_inc: {}\n\t\tignore_s_req: {}\n\t\tdec_loop_cnt: {}\n\t\treq_mode: {}\n\t\tdone_if_s_en: {}\n\t\tblock_size: {}\n\t\tbyte_swap: {}\n\t\txfer_cnt: {}\n\t\tstruct_req: {}\n\t\tstruct_type: {}\n\tSrc: 0x{:X}\n\tDst: 0x{:X}\n\tLink:\n\t\tlink: {}\n\t\tlink_mode: {}\n\t\tlink_addr: 0x{:X}\n",
            desc.ctrl.dst_mode,
            desc.ctrl.src_mode,
            desc.ctrl.dst_inc,
            desc.ctrl.size,
            desc.ctrl.src_inc,
            desc.ctrl.ignore_s_req,
            desc.ctrl.dec_loop_cnt,
            desc.ctrl.req_mode,
            desc.ctrl.done_if_s_en,
            desc.ctrl.block_size,
            desc.ctrl.byte_swap,
            desc.ctrl.xfer_cnt,
            desc.ctrl.struct_req,
            desc.ctrl.struct_type,
            desc.src,
            desc.dst,
            desc.link.link,
            desc.link.link_mode,
            desc.link.link_addr
        );
    }

    pub fn ch_enable(id: ChannelId) {
        dma().chen().modify(|_, w| unsafe { w.bits(1 << id as u8) });
    }

    pub fn ch_done(id: ChannelId) -> bool {
        dma().chdone().read().bits() & (1 << id as u8) != 0
    }

    pub fn ch_error(id: ChannelId) -> bool {
        dma().status().read().cherror().bits() == id as u8
    }

    pub fn ch_busy(id: ChannelId) -> bool {
        dma().chbusy().read().busy().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn if_clear(id: ChannelId) {
        dma().ch(id as usize).cfg().write(|w| unsafe { w.bits(0) });
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

    /// Get the DMA (pac) peripheral
    fn dma() -> Ldma {
        unsafe { crate::pac::Ldma::steal() }
    }
}
