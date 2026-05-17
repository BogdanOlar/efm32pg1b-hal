//! DMA Descriptors
//!

use crate::Sealed;
use core::marker::PhantomData;

/// XFER Descriptor builder
///
/// This descriptor defines a typical data transfer which may be a Normal, Link, or Loop transfer.
/// Only this structure type can be written directly into LDMA's registers by the CPU.
///
/// `UNIT` may be [`UnitByte`], [`UnitHalfWord`], or [`UnitWord`]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TransferDescBuilder<UNIT> {
    descr: Descriptor,
    _unit: PhantomData<UNIT>,
}

impl<UNIT: UnitTs> TransferDescBuilder<UNIT> {
    /// Build a new Transfer Descriptor
    pub const unsafe fn new(src: Addr, dst: Addr, count: TransferCount) -> Self {
        let mut descr = Descriptor::default();
        descr.struct_type_set(StructType::Transfer);
        descr.unit_size_set(UNIT::U);

        match src {
            Addr::Absolute(addr) => {
                descr.src_mode_set(AddrMode::Absolute);
                descr.src_set(addr);
            }
            Addr::Relative(offset) => {
                descr.src_mode_set(AddrMode::Relative);
                descr.src_set(offset as usize);
            }
        }

        match dst {
            Addr::Absolute(addr) => {
                descr.dst_mode_set(AddrMode::Absolute);
                descr.dst_set(addr);
            }
            Addr::Relative(offset) => {
                descr.dst_mode_set(AddrMode::Relative);
                descr.dst_set(offset as usize);
            }
        }

        descr.xfer_count_set(count.count - 1);

        Self {
            descr,
            _unit: PhantomData,
        }
    }

    pub const fn with_struct_req(mut self) -> Self {
        self.descr.struct_req_set(true);
        self
    }

    pub const fn with_block_size(mut self, block_size: BlockSize) -> Self {
        self.descr.block_size_set(block_size);
        self
    }

    pub const fn with_done_ifs(mut self) -> Self {
        self.descr.done_ifs_set(true);
        self
    }

    pub const fn with_req_mode_all(mut self) -> Self {
        self.descr.req_mode_set(true);
        self
    }

    pub const fn with_dec_loop_cnt(mut self) -> Self {
        self.descr.dec_loop_cnt_set(true);
        self
    }

    pub const fn with_ignore_single_requests(mut self) -> Self {
        self.descr.ignore_sreq_set(true);
        self
    }

    pub const fn with_src_inc(mut self, addr_inc: AddrInc) -> Self {
        self.descr.src_inc_set(addr_inc);
        self
    }

    pub const fn with_dst_inc(mut self, addr_inc: AddrInc) -> Self {
        self.descr.dst_inc_set(addr_inc);
        self
    }

    pub const fn with_link(mut self, addr: Addr) -> Self {
        self.descr.link_set(true);

        match addr {
            Addr::Absolute(addr) => {
                self.descr.link_mode_set(AddrMode::Absolute);
                self.descr.link_addr_set(addr >> 2);
            }
            Addr::Relative(offset) => {
                self.descr.link_mode_set(AddrMode::Relative);
                self.descr
                    .link_addr_set(((offset * size_of::<Descriptor>() as isize) >> 2) as usize);
            }
        }

        self
    }

    pub const fn build(self) -> Descriptor {
        self.descr
    }
}

impl<UNIT: MultiByte> TransferDescBuilder<UNIT> {
    pub const fn with_byte_swap(mut self) -> Self {
        self.descr.byte_swap_set(true);
        self
    }
}

/// DMA transfer Unit is 8-bit (typestate)
pub struct UnitByte;
/// DMA transfer Unit is 16-bit (typestate)
pub struct UnitHalfWord;
/// DMA transfer Unit is 32-bit (typestate)
pub struct UnitWord;

impl Sealed for UnitByte {}
impl Sealed for UnitHalfWord {}
impl Sealed for UnitWord {}

/// DMA transfer Unit (typestate)
pub trait UnitTs: Sealed {
    /// Unit size of DMA Transfer
    const U: UnitSize;
    /// Size of the unit, in bytes
    const BYTES: usize;
}

impl UnitTs for UnitByte {
    const U: UnitSize = UnitSize::Byte;
    const BYTES: usize = 1;
}

impl UnitTs for UnitHalfWord {
    const U: UnitSize = UnitSize::Halfword;
    const BYTES: usize = 2;
}

impl UnitTs for UnitWord {
    const U: UnitSize = UnitSize::Word;
    const BYTES: usize = 4;
}

/// DMA transfer descriptor is using HalfWord or Word unit size (marker trait)
pub trait MultiByte: Sealed {}
impl MultiByte for UnitHalfWord {}
impl MultiByte for UnitWord {}

/// SYNC Descriptor builder
///
/// This descriptor defines an intra-channel synchronizing structure.
pub struct SyncDescBuilder {
    descr: Descriptor,
}

impl SyncDescBuilder {
    /// Build a new Synchronization Descriptor
    pub const fn new() -> Self {
        let mut descr = Descriptor::default();

        descr.struct_type_set(StructType::Synchronize);
        Self { descr }
    }

    pub const fn with_syncset(mut self, bits: u8) -> Self {
        self.descr.with_syncset_set(bits);
        self
    }

    pub const fn with_syncclr(mut self, bits: u8) -> Self {
        self.descr.with_syncclr_set(bits);
        self
    }

    pub const fn with_matchen(mut self, bits: u8) -> Self {
        self.descr.with_matchen_set(bits);
        self
    }

    pub const fn with_matchval(mut self, bits: u8) -> Self {
        self.descr.with_matchval_set(bits);
        self
    }

    pub const fn with_done_ifs(mut self) -> Self {
        self.descr.done_ifs_set(true);
        self
    }

    pub const fn with_link(mut self, addr: Addr) -> Self {
        self.descr.link_set(true);

        match addr {
            Addr::Absolute(addr) => {
                self.descr.link_mode_set(AddrMode::Absolute);
                self.descr.link_addr_set(addr >> 2);
            }
            Addr::Relative(offset) => {
                self.descr.link_mode_set(AddrMode::Relative);
                self.descr
                    .link_addr_set(((offset * size_of::<Descriptor>() as isize) >> 2) as usize);
            }
        }

        self
    }

    pub const fn build(self) -> Descriptor {
        self.descr
    }
}

/// WRI Descriptor builder
///
/// This descriptor defines a write-immediate structure.
pub struct ImmediateDescBuilder {
    descr: Descriptor,
}

impl ImmediateDescBuilder {
    /// Build a new Immediate Write Descriptor
    pub const unsafe fn new(val: u32, dst: usize) -> Self {
        let mut descr = Descriptor::default();

        descr.struct_type_set(StructType::Write);
        descr.val_set(val);
        descr.dst_set(dst);

        Self { descr }
    }

    pub const fn with_done_ifs(mut self) -> Self {
        self.descr.done_ifs_set(true);
        self
    }

    pub const fn with_link(mut self, addr: Addr) -> Self {
        self.descr.link_set(true);

        match addr {
            Addr::Absolute(addr) => {
                self.descr.link_mode_set(AddrMode::Absolute);
                self.descr.link_addr_set(addr >> 2);
            }
            Addr::Relative(offset) => {
                self.descr.link_mode_set(AddrMode::Relative);
                self.descr
                    .link_addr_set(((offset * size_of::<Descriptor>() as isize) >> 2) as usize);
            }
        }

        self
    }

    pub const fn build(self) -> Descriptor {
        self.descr
    }
}

/// DMA Descriptor
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct Descriptor {
    pub(crate) raw: [u32; 4],
}

impl Descriptor {
    /// Maximum number of units (byte, half-word, word) which can be transfered in one DMA shot
    #[cfg(not(feature = "dma-debug-max-transfer"))]
    pub const MAX_TRANSFER_UNITS: usize = 1 << 11;
    #[cfg(feature = "dma-debug-max-transfer")]
    pub const MAX_TRANSFER_UNITS: usize = 1 << 5;

    pub(crate) const INDEX_CTRL: usize = 0;
    pub(crate) const INDEX_SRC: usize = 1;
    pub(crate) const INDEX_DST: usize = 2;
    pub(crate) const INDEX_LINK: usize = 3;

    const CTRL_STRUCT_TYPE_OFFSET: usize = 0;
    const CTRL_STRUCT_TYPE_MASK: u32 = 0b11;
    const CTRL_STRUCT_REQ_OFFSET: usize = 3;
    const CTRL_STRUCT_REQ_MASK: u32 = 0b1;
    const CTRL_XFER_COUNT_OFFSET: usize = 4;
    const CTRL_XFER_COUNT_MASK: u32 = (Self::MAX_TRANSFER_UNITS - 1) as u32;
    const CTRL_BYTE_SWAP_OFFSET: usize = 15;
    const CTRL_BYTE_SWAP_MASK: u32 = 0b1;
    const CTRL_BLOCK_SIZE_OFFSET: usize = 16;
    const CTRL_BLOCK_SIZE_MASK: u32 = 0b1111;
    const CTRL_DONE_IFS_EN_OFFSET: usize = 20;
    const CTRL_DONE_IFS_EN_MASK: u32 = 0b1;
    const CTRL_REQ_MODE_OFFSET: usize = 21;
    const CTRL_REQ_MODE_MASK: u32 = 0b1;
    const CTRL_DEC_LOOP_CNT_OFFSET: usize = 22;
    const CTRL_DEC_LOOP_CNT_MASK: u32 = 0b1;
    const CTRL_IGNORE_SREQ_OFFSET: usize = 23;
    const CTRL_IGNORE_SREQ_MASK: u32 = 0b1;
    const CTRL_SRC_INC_OFFSET: usize = 24;
    const CTRL_SRC_INC_MASK: u32 = 0b11;
    const CTRL_UNIT_SIZE_OFFSET: usize = 26;
    const CTRL_UNIT_SIZE_MASK: u32 = 0b11;
    const CTRL_DST_INC_OFFSET: usize = 28;
    const CTRL_DST_INC_MASK: u32 = 0b11;
    const CTRL_SRC_MODE_OFFSET: usize = 30;
    const CTRL_SRC_MODE_MASK: u32 = 0b1;
    const CTRL_DST_MODE_OFFSET: usize = 31;
    const CTRL_DST_MODE_MASK: u32 = 0b1;

    const SRC_SYNCSET_OFFSET: usize = 0;
    const SRC_SYNCSET_MASK: u32 = 0xFF;
    const SRC_SYNCCLR_OFFSET: usize = 8;
    const SRC_SYNCCLR_MASK: u32 = 0xFF;

    const DST_MATCHVAL_OFFSET: usize = 0;
    const DST_MATCHVAL_MASK: u32 = 0xFF;
    const DST_MATCHEN_OFFSET: usize = 8;
    const DST_MATCHEN_MASK: u32 = 0xFF;

    const LINK_MODE_OFFSET: usize = 0;
    const LINK_MODE_MASK: u32 = 0b1;
    const LINK_OFFSET: usize = 1;
    const LINK_MASK: u32 = 0b1;
    const LINK_ADDR_OFFSET: usize = 2;
    const LINK_ADDR_MASK: u32 = 0x3FFFFFFF;

    /// Const default for [`Descriptor`]
    const fn default() -> Self {
        Self { raw: [0; 4] }
    }

    pub(crate) const fn struct_type_set(&mut self, struct_type: StructType) {
        self.raw[Self::INDEX_CTRL] &=
            !(Self::CTRL_STRUCT_TYPE_MASK << Self::CTRL_STRUCT_TYPE_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (struct_type as u32 & Self::CTRL_STRUCT_TYPE_MASK) << Self::CTRL_STRUCT_TYPE_OFFSET;
    }

    pub(crate) const fn struct_req_set(&mut self, do_struct_req: bool) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_STRUCT_REQ_MASK << Self::CTRL_STRUCT_REQ_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (do_struct_req as u32 & Self::CTRL_STRUCT_REQ_MASK) << Self::CTRL_STRUCT_REQ_OFFSET;
    }

    pub(crate) const fn xfer_count_set(&mut self, count: u16) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_XFER_COUNT_MASK << Self::CTRL_XFER_COUNT_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (count as u32 & Self::CTRL_XFER_COUNT_MASK) << Self::CTRL_XFER_COUNT_OFFSET;
    }

    pub(crate) const fn byte_swap_set(&mut self, do_byte_swap: bool) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_BYTE_SWAP_MASK << Self::CTRL_BYTE_SWAP_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (do_byte_swap as u32 & Self::CTRL_BYTE_SWAP_MASK) << Self::CTRL_BYTE_SWAP_OFFSET;
    }

    pub(crate) const fn block_size_set(&mut self, block_size: BlockSize) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_BLOCK_SIZE_MASK << Self::CTRL_BLOCK_SIZE_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (block_size as u32 & Self::CTRL_BLOCK_SIZE_MASK) << Self::CTRL_BLOCK_SIZE_OFFSET;
    }

    pub(crate) const fn done_ifs_set(&mut self, flag: bool) {
        self.raw[Self::INDEX_CTRL] &=
            !(Self::CTRL_DONE_IFS_EN_MASK << Self::CTRL_DONE_IFS_EN_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (flag as u32 & Self::CTRL_DONE_IFS_EN_MASK) << Self::CTRL_DONE_IFS_EN_OFFSET;
    }

    pub(crate) const fn req_mode_set(&mut self, all: bool) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_REQ_MODE_MASK << Self::CTRL_REQ_MODE_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (all as u32 & Self::CTRL_REQ_MODE_MASK) << Self::CTRL_REQ_MODE_OFFSET;
    }

    pub(crate) const fn dec_loop_cnt_set(&mut self, decrement_loop: bool) {
        self.raw[Self::INDEX_CTRL] &=
            !(Self::CTRL_DEC_LOOP_CNT_MASK << Self::CTRL_DEC_LOOP_CNT_OFFSET);
        self.raw[Self::INDEX_CTRL] |= (decrement_loop as u32 & Self::CTRL_DEC_LOOP_CNT_MASK)
            << Self::CTRL_DEC_LOOP_CNT_OFFSET;
    }

    pub(crate) const fn ignore_sreq_set(&mut self, ignore_srec: bool) {
        self.raw[Self::INDEX_CTRL] &=
            !(Self::CTRL_IGNORE_SREQ_MASK << Self::CTRL_IGNORE_SREQ_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (ignore_srec as u32 & Self::CTRL_IGNORE_SREQ_MASK) << Self::CTRL_IGNORE_SREQ_OFFSET;
    }

    pub(crate) const fn src_inc_set(&mut self, addr_inc: AddrInc) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_SRC_INC_MASK << Self::CTRL_SRC_INC_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (addr_inc as u32 & Self::CTRL_SRC_INC_MASK) << Self::CTRL_SRC_INC_OFFSET;
    }

    pub(crate) const fn unit_size_set(&mut self, unit_size: UnitSize) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_UNIT_SIZE_MASK << Self::CTRL_UNIT_SIZE_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (unit_size as u32 & Self::CTRL_UNIT_SIZE_MASK) << Self::CTRL_UNIT_SIZE_OFFSET;
    }

    pub(crate) const fn dst_inc_set(&mut self, addr_inc: AddrInc) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_DST_INC_MASK << Self::CTRL_DST_INC_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (addr_inc as u32 & Self::CTRL_DST_INC_MASK) << Self::CTRL_DST_INC_OFFSET;
    }

    pub(crate) const fn src_mode_set(&mut self, addr_mode: AddrMode) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_SRC_MODE_MASK << Self::CTRL_SRC_MODE_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (addr_mode as u32 & Self::CTRL_SRC_MODE_MASK) << Self::CTRL_SRC_MODE_OFFSET;
    }

    pub(crate) const fn dst_mode_set(&mut self, addr_mode: AddrMode) {
        self.raw[Self::INDEX_CTRL] &= !(Self::CTRL_DST_MODE_MASK << Self::CTRL_DST_MODE_OFFSET);
        self.raw[Self::INDEX_CTRL] |=
            (addr_mode as u32 & Self::CTRL_DST_MODE_MASK) << Self::CTRL_DST_MODE_OFFSET;
    }

    pub(crate) const fn src_set(&mut self, src: usize) {
        self.raw[Self::INDEX_SRC] = src as u32;
    }

    pub(crate) const fn val_set(&mut self, val: u32) {
        self.raw[Self::INDEX_SRC] = val;
    }

    pub(crate) const fn dst_set(&mut self, dst: usize) {
        self.raw[Self::INDEX_DST] = dst as u32;
    }

    pub(crate) const fn link_mode_set(&mut self, addr_mode: AddrMode) {
        self.raw[Self::INDEX_LINK] &= !(Self::LINK_MODE_MASK << Self::LINK_MODE_OFFSET);
        self.raw[Self::INDEX_LINK] |=
            (addr_mode as u32 & Self::LINK_MODE_MASK) << Self::LINK_MODE_OFFSET;
    }

    pub(crate) const fn link_set(&mut self, is_link: bool) {
        self.raw[Self::INDEX_LINK] &= !(Self::LINK_MASK << Self::LINK_OFFSET);
        self.raw[Self::INDEX_LINK] |= (is_link as u32 & Self::LINK_MASK) << Self::LINK_OFFSET;
    }

    pub(crate) const fn link_addr_set(&mut self, addr: usize) {
        self.raw[Self::INDEX_LINK] &= !(Self::LINK_ADDR_MASK << Self::LINK_ADDR_OFFSET);
        self.raw[Self::INDEX_LINK] |=
            (addr as u32 & Self::LINK_ADDR_MASK) << Self::LINK_ADDR_OFFSET;
    }

    pub(crate) const fn with_syncset_set(&mut self, bits: u8) {
        self.raw[Self::INDEX_SRC] &= !(Self::SRC_SYNCSET_MASK << Self::SRC_SYNCSET_OFFSET);
        self.raw[Self::INDEX_SRC] |=
            (bits as u32 & Self::SRC_SYNCSET_MASK) << Self::SRC_SYNCSET_OFFSET;
    }

    pub(crate) const fn with_syncclr_set(&mut self, bits: u8) {
        self.raw[Self::INDEX_SRC] &= !(Self::SRC_SYNCCLR_MASK << Self::SRC_SYNCCLR_OFFSET);
        self.raw[Self::INDEX_SRC] |=
            (bits as u32 & Self::SRC_SYNCCLR_MASK) << Self::SRC_SYNCCLR_OFFSET;
    }

    pub(crate) const fn with_matchval_set(&mut self, bits: u8) {
        self.raw[Self::INDEX_DST] &= !(Self::DST_MATCHVAL_MASK << Self::DST_MATCHVAL_OFFSET);
        self.raw[Self::INDEX_DST] |=
            (bits as u32 & Self::DST_MATCHVAL_MASK) << Self::DST_MATCHVAL_OFFSET;
    }

    pub(crate) const fn with_matchen_set(&mut self, bits: u8) {
        self.raw[Self::INDEX_DST] &= !(Self::DST_MATCHEN_MASK << Self::DST_MATCHEN_OFFSET);
        self.raw[Self::INDEX_DST] |=
            (bits as u32 & Self::DST_MATCHEN_MASK) << Self::DST_MATCHEN_OFFSET;
    }
}

/// Address
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Addr {
    /// Absolute address
    Absolute(usize),
    /// Relative addres
    Relative(isize),
}

///Source/Destination Address Increment Size
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AddrInc {
    ///0: Increment source/destination address by one unit data size after each read/write
    One = 0,
    ///1: Increment source/destination address by two unit data sizes after each read/write
    Two = 1,
    ///2: Increment source/destination address by four unit data sizes after each read/write
    Four = 2,
    ///3: Do not increment the source/destination address. Read/writes are made to a fixed destination address, for
    /// example writing to a FIFO.
    None = 3,
}

/// Unit Data Transfer Size
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum UnitSize {
    /// 8-bit transfer unit
    Byte = 0,
    /// 16-bit transfer unit
    Halfword = 1,
    /// 32-bit transfer unit
    Word = 2,
}

/// Descriptor address mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum AddrMode {
    /// Absolute addressing
    #[default]
    Absolute = 0,
    /// Relative addressing
    Relative = 1,
}

///Block Transfer Size
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockSize {
    ///0: One unit transfer per arbitration
    Unit1 = 0,
    ///1: Two unit transfers per arbitration
    Unit2 = 1,
    ///2: Three unit transfers per arbitration
    Unit3 = 2,
    ///3: Four unit transfers per arbitration
    Unit4 = 3,
    ///4: Six unit transfers per arbitration
    Unit6 = 4,
    ///5: Eight unit transfers per arbitration
    Unit8 = 5,
    ///7: Sixteen unit transfers per arbitration
    Unit16 = 7,
    ///9: 32 unit transfers per arbitration
    Unit32 = 9,
    ///10: 64 unit transfers per arbitration
    Unit64 = 10,
    ///11: 128 unit transfers per arbitration
    Unit128 = 11,
    ///12: 256 unit transfers per arbitration
    Unit256 = 12,
    ///13: 512 unit transfers per arbitration
    Unit512 = 13,
    ///14: 1024 unit transfers per arbitration
    Unit1024 = 14,
    ///15: Transfer all units as specified by the XFRCNT field
    All = 15,
}

/// DMA Structure Type
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum StructType {
    ///0: DMA transfer structure type selected.
    Transfer = 0,
    ///1: Synchronization structure type selected.
    Synchronize = 1,
    ///2: Write immediate value structure type selected.
    Write = 2,
}

/// Number of units (byte, half-word, or word) which can be transfered with a Transfer Descriptor.
///
/// Ensures the value is at most `Descriptor::MAX_TRANSFER_UNITS`, and not `0`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TransferCount {
    count: u16,
}

impl TryFrom<usize> for TransferCount {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if (value > 0) && (value <= Descriptor::MAX_TRANSFER_UNITS) {
            Ok(Self {
                count: value as u16,
            })
        } else {
            Err(())
        }
    }
}
