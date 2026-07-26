/// Descriptor list and the Descriptor types which can be used with it
///
use crate::dma::{
    descriptor::{
        Addr, AddrInc, AddrMode, BlockSize, Descriptor, RawLoopTransferDescBuilder,
        RawTransferDescBuilder, StructType, TransferCount, UnitSize,
    },
    DmaError,
};
use core::slice::IterMut;

pub struct DescList<'a> {
    prev: Option<(ListDescriptor, &'a mut Descriptor)>,
    desc_iter: IterMut<'a, Descriptor>,
}

impl<'a> DescList<'a> {
    pub fn new(storage: &'a mut [Descriptor]) -> Self {
        storage.iter_mut().for_each(|d| *d = Descriptor::default());
        Self {
            prev: None,
            desc_iter: storage.into_iter(),
        }
    }

    pub fn push<T>(mut self, desc_bld: T) -> Result<Self, DmaError>
    where
        T: Into<ListDescriptor> + Copy,
    {
        let desc = self
            .desc_iter
            .next()
            .ok_or(DmaError::DescriptorListOverflow)?;

        self.link_prev();

        *desc = match desc_bld.into() {
            ListDescriptor::Transfer(desc_bld) => desc_bld.build(),
            ListDescriptor::LoopTransfer(desc_bld) => desc_bld.build(),
            ListDescriptor::Sync(desc_bld) => desc_bld.build(),
            ListDescriptor::Immediate(desc_bld) => desc_bld.build(),
        };
        Ok(Self {
            prev: Some((desc_bld.into(), desc)),
            desc_iter: self.desc_iter,
        })
    }

    fn link_prev(&mut self) {
        if let Some((prev_builder, prev_descr)) = self.prev.take() {
            *prev_descr = match prev_builder {
                ListDescriptor::Transfer(transfer_desc_builder) => transfer_desc_builder
                    .inner
                    .with_link(Addr::Relative(1))
                    .build(),
                ListDescriptor::LoopTransfer(loop_transfer_desc_builder) => {
                    loop_transfer_desc_builder.inner.with_link(true).build()
                }
                ListDescriptor::Sync(sync_desc_builder) => {
                    sync_desc_builder.with_link(Addr::Relative(1)).build()
                }
                ListDescriptor::Immediate(immediate_desc_builder) => {
                    immediate_desc_builder.with_link(Addr::Relative(1)).build()
                }
            };
        }
    }
}

enum ListDescriptor {
    Transfer(TransferDescBuilder),
    LoopTransfer(LoopTransferDescBuilder),
    Sync(SyncDescBuilder),
    Immediate(ImmediateDescBuilder),
}

impl From<TransferDescBuilder> for ListDescriptor {
    fn from(value: TransferDescBuilder) -> Self {
        Self::Transfer(value)
    }
}

impl From<LoopTransferDescBuilder> for ListDescriptor {
    fn from(value: LoopTransferDescBuilder) -> Self {
        Self::LoopTransfer(value)
    }
}

impl From<SyncDescBuilder> for ListDescriptor {
    fn from(value: SyncDescBuilder) -> Self {
        Self::Sync(value)
    }
}

impl From<ImmediateDescBuilder> for ListDescriptor {
    fn from(value: ImmediateDescBuilder) -> Self {
        Self::Immediate(value)
    }
}

/// XFER Descriptor builder
///
/// This descriptor defines a typical data transfer which may be a Normal or Link transfer.
///
/// This descriptor can be written directly into LDMA's registers
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Default, Clone, Copy)]
pub struct TransferDescBuilder {
    inner: RawTransferDescBuilder,
}

impl TransferDescBuilder {
    /// Build a new Transfer Descriptor
    pub const fn new(src: Addr, dst: Addr, count: TransferCount, unit: UnitSize) -> Self {
        Self {
            inner: RawTransferDescBuilder::new(src, dst, count, unit),
        }
    }

    pub const fn with_struct_req(self) -> Self {
        Self {
            inner: self.inner.with_struct_req(),
        }
    }

    pub const fn with_block_size(self, block_size: BlockSize) -> Self {
        Self {
            inner: self.inner.with_block_size(block_size),
        }
    }

    pub const fn with_byte_swap(self) -> Self {
        Self {
            inner: self.inner.with_byte_swap(),
        }
    }

    /// Setting this bit will set the interrupt flag when the transfer is done, or linked in the case where the LINK bit
    /// is set, or synchronized in the case of a SYNC transfer.
    pub const fn with_done_ifs(self, is_done: bool) -> Self {
        Self {
            inner: self.inner.with_done_ifs(is_done),
        }
    }

    pub const fn with_req_mode_all(self) -> Self {
        Self {
            inner: self.inner.with_req_mode_all(),
        }
    }

    pub const fn with_ignore_single_requests(self) -> Self {
        Self {
            inner: self.inner.with_ignore_single_requests(),
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
/// This descriptor can be written directly into LDMA's registers
///
/// # WARNING
///
/// if this descriptor is linked (by calling [`LoopTransferDescBuilder::with_link()`] with `true`), then
///          once the DMA channel counter reaches `0`, the *next* descriptor in the list will be executed
///
/// # TODO
/// constrain the [`DescList`] to never end with a Loop descriptor which is Link
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Default, Clone, Copy)]
pub struct LoopTransferDescBuilder {
    inner: RawLoopTransferDescBuilder,
}

impl LoopTransferDescBuilder {
    /// Build a new Transfer Descriptor
    pub const fn new(src: Addr, dst: Addr, count: TransferCount, unit: UnitSize) -> Self {
        Self {
            inner: RawLoopTransferDescBuilder::new(src, dst, count, unit),
        }
    }

    pub const fn with_struct_req(self) -> Self {
        Self {
            inner: self.inner.with_struct_req(),
        }
    }

    pub const fn with_block_size(self, block_size: BlockSize) -> Self {
        Self {
            inner: self.inner.with_block_size(block_size),
        }
    }

    pub const fn with_byte_swap(self) -> Self {
        Self {
            inner: self.inner.with_byte_swap(),
        }
    }

    pub const fn with_loop_done_ifs(self, is_done: bool) -> Self {
        Self {
            inner: self.inner.with_loop_done_ifs(is_done),
        }
    }

    pub const fn with_req_mode_all(self) -> Self {
        Self {
            inner: self.inner.with_req_mode_all(),
        }
    }

    pub const fn with_ignore_single_requests(self) -> Self {
        Self {
            inner: self.inner.with_ignore_single_requests(),
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

    /// Set the link register
    ///
    /// The link `flag` needs to be specified separately since we can have looped descriptors which
    pub const fn with_loop(self, addr: Addr) -> Self {
        Self {
            inner: self.inner.with_loop(addr),
        }
    }

    pub(crate) const fn build(self) -> Descriptor {
        self.inner.build()
    }
}

/// SYNC Descriptor builder
///
/// This descriptor defines an intra-channel synchronizing structure.
///
/// This descriptor can only be linked from memory, not written directly to the DMA channel registers
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

    const fn with_link(mut self, addr: Addr) -> Self {
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
///
/// This descriptor can only be linked from memory, not written directly to the DMA channel registers
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

    const fn with_link(mut self, addr: Addr) -> Self {
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
