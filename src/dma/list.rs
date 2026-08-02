/// Descriptor list and the Descriptor types which can be used with it
///
use crate::dma::{
    descriptor::{
        Addr, AddrInc, AddrMode, BlockSize, Descriptor, StructType, TransferCount,
        TransferDescriptor, UnitSize,
    },
    DmaError,
};
use core::slice::IterMut;

/// A descriptor list capable of storing descriptors and automatically linking them
pub struct DescList<'a> {
    prev: Option<(ListDescriptorBuilder, &'a mut Descriptor)>,
    desc_iter: IterMut<'a, Descriptor>,
    link_descriptor: Descriptor,
    storage_addr: usize,
}

impl<'a> DescList<'a> {
    /// Create a new Descriptor List using the given `storage`
    pub fn new(storage: &'a mut [Descriptor]) -> Self {
        let storage_addr = storage.as_ptr().addr();
        storage.iter_mut().for_each(|d| *d = Descriptor::default());
        Self {
            prev: None,
            desc_iter: storage.iter_mut(),
            link_descriptor: LinkDescriptorBuilder::new(storage_addr).build(),
            storage_addr,
        }
    }

    /// Convenience method to [`Self::push()`] [`TransferDescBuilder`] to the descriptor list
    pub fn push_transfer(self, desc_bld: TransferDescBuilder) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`LoopTransferDescBuilder`] to the descriptor list
    pub fn push_loop(self, desc_bld: LoopTransferDescBuilder) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`SyncDescBuilder`] to the descriptor list
    pub fn push_sync(self, desc_bld: SyncDescBuilder) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`ImmediateDescBuilder`] to the descriptor list
    pub fn push_immediate(self, desc_bld: ImmediateDescBuilder) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Generic method to push any of the descriptor builders wrapped by [`ListDescriptor`]
    pub fn push<T>(mut self, desc_bld: T) -> Result<Self, DmaError>
    where
        T: Into<ListDescriptorBuilder> + Copy,
    {
        let desc = self
            .desc_iter
            .next()
            .ok_or(DmaError::DescriptorListOverflow)?;

        self.link_prev();

        *desc = match desc_bld.into() {
            ListDescriptorBuilder::Transfer(desc_bld) => desc_bld.build(),
            ListDescriptorBuilder::LoopTransfer(desc_bld) => desc_bld.build(),
            ListDescriptorBuilder::Sync(desc_bld) => desc_bld.build(),
            ListDescriptorBuilder::Immediate(desc_bld) => desc_bld.build(),
        };
        Ok(Self {
            prev: Some((desc_bld.into(), desc)),
            desc_iter: self.desc_iter,
            link_descriptor: self.link_descriptor,
            storage_addr: self.storage_addr,
        })
    }

    pub fn finalize(self) -> Descriptor {
        self.link_descriptor
    }

    pub(crate) fn storage_addr(&self) -> usize {
        self.storage_addr
    }

    /// Modify the previous descriptor (if it exists) to link to the next descriptor in the list
    fn link_prev(&mut self) {
        if let Some((prev_builder, prev_descr)) = self.prev.take() {
            *prev_descr = match prev_builder {
                ListDescriptorBuilder::Transfer(transfer_desc_builder) => transfer_desc_builder
                    .inner
                    .with_link(Addr::Relative(1), true)
                    .build(),
                ListDescriptorBuilder::LoopTransfer(loop_transfer_desc_builder) => {
                    loop_transfer_desc_builder.with_link(true).build()
                }
                ListDescriptorBuilder::Sync(sync_desc_builder) => {
                    sync_desc_builder.with_link(Addr::Relative(1), true).build()
                }
                ListDescriptorBuilder::Immediate(immediate_desc_builder) => immediate_desc_builder
                    .with_link(Addr::Relative(1), true)
                    .build(),
            };
        }
    }
}

/// Wrapper for all Descriptor Builders which can be pushed to a [`DescList`]
pub enum ListDescriptorBuilder {
    /// [`TransferDescBuilder`]
    Transfer(TransferDescBuilder),
    /// [`LoopTransferDescBuilder`]
    LoopTransfer(LoopTransferDescBuilder),
    /// [`SyncDescBuilder`]
    Sync(SyncDescBuilder),
    /// [`ImmediateDescBuilder`]
    Immediate(ImmediateDescBuilder),
}

impl From<TransferDescBuilder> for ListDescriptorBuilder {
    fn from(value: TransferDescBuilder) -> Self {
        Self::Transfer(value)
    }
}

impl From<LoopTransferDescBuilder> for ListDescriptorBuilder {
    fn from(value: LoopTransferDescBuilder) -> Self {
        Self::LoopTransfer(value)
    }
}

impl From<SyncDescBuilder> for ListDescriptorBuilder {
    fn from(value: SyncDescBuilder) -> Self {
        Self::Sync(value)
    }
}

impl From<ImmediateDescBuilder> for ListDescriptorBuilder {
    fn from(value: ImmediateDescBuilder) -> Self {
        Self::Immediate(value)
    }
}

/// XFER Descriptor builder
///
/// This descriptor defines a typical data transfer which may be a Normal or Link transfer.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Default, Clone, Copy)]
pub struct TransferDescBuilder {
    inner: TransferDescriptor,
}

impl TransferDescBuilder {
    /// Build a new Transfer Descriptor
    pub const fn new(src: Addr, dst: Addr, count: TransferCount, unit: UnitSize) -> Self {
        Self {
            inner: TransferDescriptor::new(src, dst, count, unit),
        }
    }

    pub const fn with_struct_req(self, is_struct_req: bool) -> Self {
        Self {
            inner: self.inner.with_struct_req(is_struct_req),
        }
    }

    pub const fn with_block_size(self, block_size: BlockSize) -> Self {
        Self {
            inner: self.inner.with_block_size(block_size),
        }
    }

    pub const fn with_byte_swap(self, is_byte_swapped: bool) -> Self {
        Self {
            inner: self.inner.with_byte_swap(is_byte_swapped),
        }
    }

    /// Setting this bit will set the interrupt flag when the transfer is done, or linked in the case where the LINK bit
    /// is set, or synchronized in the case of a SYNC transfer.
    pub const fn with_done_ifs(self, is_done: bool) -> Self {
        Self {
            inner: self.inner.with_done_ifs(is_done),
        }
    }

    pub const fn with_req_mode_all(self, is_req_mode_all: bool) -> Self {
        Self {
            inner: self.inner.with_req_mode_all(is_req_mode_all),
        }
    }

    pub const fn with_ignore_single_requests(self, is_ignored: bool) -> Self {
        Self {
            inner: self.inner.with_ignore_single_requests(is_ignored),
        }
    }

    pub const fn with_src_inc(self, addr_inc: AddrInc) -> Self {
        Self {
            inner: self.inner.with_src_inc(addr_inc),
        }
    }

    pub const fn with_dst_inc(self, addr_inc: AddrInc) -> Self {
        Self {
            inner: self.inner.with_dst_inc(addr_inc),
        }
    }

    pub(crate) const fn build(self) -> Descriptor {
        self.inner.build()
    }
}

/// Looped XFER Descriptor builder
///
/// This descriptor defines a typical data transfer which may be a Loop or Link transfer.
///
/// # TODO
/// constrain the [`DescList`] to never end with a Loop descriptor which is Link
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Default, Clone, Copy)]
pub struct LoopTransferDescBuilder {
    descr: Descriptor,
}

impl LoopTransferDescBuilder {
    /// Build a new Transfer Descriptor
    pub const fn new(
        src: Addr,
        dst: Addr,
        count: TransferCount,
        loop_to: Addr,
        unit: UnitSize,
    ) -> Self {
        let mut descr = Descriptor::const_default();
        descr.struct_type_set(StructType::Transfer);
        descr.dec_loop_cnt_set(true);
        descr.unit_size_set(unit);

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

        match loop_to {
            Addr::Absolute(addr) => {
                descr.link_mode_set(AddrMode::Absolute);
                descr.link_addr_set(addr >> 2);
            }
            Addr::Relative(offset) => {
                descr.link_mode_set(AddrMode::Relative);
                descr.link_addr_set(((offset * size_of::<Descriptor>() as isize) >> 2) as usize);
            }
        }

        descr.xfer_count_set(count.count - 1);

        Self { descr }
    }

    pub const fn with_struct_req(mut self) -> Self {
        self.descr.struct_req_set(true);
        self
    }

    pub const fn with_block_size(mut self, block_size: BlockSize) -> Self {
        self.descr.block_size_set(block_size);
        self
    }

    pub const fn with_byte_swap(mut self) -> Self {
        self.descr.byte_swap_set(true);
        self
    }

    pub const fn with_loop_done_ifs(mut self, is_done: bool) -> Self {
        self.descr.done_ifs_set(is_done);
        self
    }

    pub const fn with_req_mode_all(mut self) -> Self {
        self.descr.req_mode_set(true);
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

    /// Use this flag to determine if another descriptor (placed immediatelly after it) needs to be loaded if the
    /// DMA channel loop counter reaches `0`
    const fn with_link(mut self, is_linked: bool) -> Self {
        self.descr.link_set(is_linked);
        self
    }

    pub const fn build(self) -> Descriptor {
        self.descr
    }
}

/// SYNC Descriptor builder
///
/// This descriptor defines an intra-channel synchronizing structure.
///
/// This descriptor can only be linked from memory (e.g written to a [`DescList`])
#[derive(Default, Clone, Copy)]
pub struct SyncDescBuilder {
    descr: Descriptor,
}

impl SyncDescBuilder {
    /// Build a new Synchronization Descriptor
    pub const fn new() -> Self {
        let mut descr = Descriptor::const_default();

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

    pub const fn with_done_ifs(mut self, is_done: bool) -> Self {
        self.descr.done_ifs_set(is_done);
        self
    }

    const fn with_link(mut self, addr: Addr, is_linked: bool) -> Self {
        self.descr.link_set(is_linked);

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
///
/// This descriptor can only be linked from memory (e.g written to a [`DescList`])
#[derive(Default, Clone, Copy)]
pub struct ImmediateDescBuilder {
    descr: Descriptor,
}

impl ImmediateDescBuilder {
    /// Build a new Immediate Write Descriptor
    pub const fn new(val: u32, dst: usize) -> Self {
        let mut descr = Descriptor::const_default();

        descr.struct_type_set(StructType::Write);
        descr.val_set(val);
        descr.dst_set(dst);

        Self { descr }
    }

    pub const fn with_done_ifs(mut self, is_done: bool) -> Self {
        self.descr.done_ifs_set(is_done);
        self
    }

    const fn with_link(mut self, addr: Addr, is_linked: bool) -> Self {
        self.descr.link_set(is_linked);

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

pub struct LinkDescriptorBuilder {
    descr: Descriptor,
}

impl LinkDescriptorBuilder {
    const fn new(addr: usize) -> Self {
        let mut descr = Descriptor::const_default();
        descr.struct_type_set(StructType::Transfer);

        descr.link_mode_set(AddrMode::Absolute);
        descr.link_addr_set(addr >> 2);

        Self { descr }
    }

    pub const fn build(self) -> Descriptor {
        self.descr
    }
}
