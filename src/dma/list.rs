/// Descriptor list and the Descriptor types whic can be used with it
///
use crate::dma::{
    descriptor::{
        Addr, AddrInc, BlockSize, Descriptor, ImmediateDescBuilder, LoopTransferDescBuilder,
        SyncDescBuilder, TransferCount, TransferDescBuilder, UnitSize,
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
                    sync_desc_builder.inner.with_link(Addr::Relative(1)).build()
                }
                ListDescriptor::Immediate(immediate_desc_builder) => immediate_desc_builder
                    .inner
                    .with_link(Addr::Relative(1))
                    .build(),
            };
        }
    }
}

enum ListDescriptor {
    Transfer(TransferListDescBuilder),
    LoopTransfer(LoopTransferListDescBuilder),
    Sync(SyncListDescBuilder),
    Immediate(ImmediateListDescBuilder),
}

impl From<TransferListDescBuilder> for ListDescriptor {
    fn from(value: TransferListDescBuilder) -> Self {
        Self::Transfer(value)
    }
}

impl From<LoopTransferListDescBuilder> for ListDescriptor {
    fn from(value: LoopTransferListDescBuilder) -> Self {
        Self::LoopTransfer(value)
    }
}

impl From<SyncListDescBuilder> for ListDescriptor {
    fn from(value: SyncListDescBuilder) -> Self {
        Self::Sync(value)
    }
}

impl From<ImmediateListDescBuilder> for ListDescriptor {
    fn from(value: ImmediateListDescBuilder) -> Self {
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
pub struct TransferListDescBuilder {
    inner: TransferDescBuilder,
}

impl TransferListDescBuilder {
    /// Build a new Transfer Descriptor
    pub const fn new(src: Addr, dst: Addr, count: TransferCount, unit: UnitSize) -> Self {
        Self {
            inner: TransferDescBuilder::new(src, dst, count, unit),
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
pub struct LoopTransferListDescBuilder {
    inner: LoopTransferDescBuilder,
}

impl LoopTransferListDescBuilder {
    /// Build a new Transfer Descriptor
    pub const fn new(src: Addr, dst: Addr, count: TransferCount, unit: UnitSize) -> Self {
        Self {
            inner: LoopTransferDescBuilder::new(src, dst, count, unit),
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
pub struct SyncListDescBuilder {
    inner: SyncDescBuilder,
}

impl SyncListDescBuilder {
    /// Build a new Synchronization Descriptor
    pub const fn new() -> Self {
        Self {
            inner: SyncDescBuilder::new(),
        }
    }

    pub const fn with_syncset(self, bits: u8) -> Self {
        Self {
            inner: self.inner.with_syncset(bits),
        }
    }

    pub const fn with_syncclr(self, bits: u8) -> Self {
        Self {
            inner: self.inner.with_syncclr(bits),
        }
    }

    pub const fn with_matchen(self, bits: u8) -> Self {
        Self {
            inner: self.inner.with_matchen(bits),
        }
    }

    pub const fn with_matchval(self, bits: u8) -> Self {
        Self {
            inner: self.inner.with_matchval(bits),
        }
    }

    pub const fn with_done_ifs(self, is_done: bool) -> Self {
        Self {
            inner: self.inner.with_done_ifs(is_done),
        }
    }

    pub(crate) const fn build(self) -> Descriptor {
        self.inner.build()
    }
}

/// WRI Descriptor builder
///
/// This descriptor defines a write-immediate structure.
///
/// This descriptor can only be linked from memory, not written directly to the DMA channel registers
#[derive(Default, Clone, Copy)]
pub struct ImmediateListDescBuilder {
    inner: ImmediateDescBuilder,
}

impl ImmediateListDescBuilder {
    /// Build a new Immediate Write Descriptor
    pub const fn new(val: u32, dst: usize) -> Self {
        Self {
            inner: ImmediateDescBuilder::new(val, dst),
        }
    }

    pub const fn with_done_ifs(self, is_done: bool) -> Self {
        Self {
            inner: self.inner.with_done_ifs(is_done),
        }
    }

    pub(crate) const fn build(self) -> Descriptor {
        self.inner.build()
    }
}
