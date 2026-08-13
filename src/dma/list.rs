/// Descriptor list and the Descriptor types which can be used with it
///
use crate::dma::{
    descriptor::{
        Addr, Descriptor, ImmediateDescriptor, LinkDescriptorBuilder, LoopTransferDescriptor,
        SyncDescriptor, TransferDescriptor,
    },
    DmaError,
};
use core::slice::IterMut;

/// A descriptor list capable of storing descriptors and automatically linking them
pub struct DescList<'a> {
    prev: Option<(ListDescriptor, &'a mut Descriptor)>,
    descriptors: IterMut<'a, Descriptor>,
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
            descriptors: storage.iter_mut(),
            link_descriptor: LinkDescriptorBuilder::new(storage_addr).into_inner(),
            storage_addr,
        }
    }

    /// Convenience method to [`Self::push()`] [`TransferDescBuilder`] to the descriptor list
    pub fn push_transfer(self, desc_bld: TransferDescriptor) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`LoopTransferDescBuilder`] to the descriptor list
    pub fn push_loop(self, desc_bld: LoopTransferDescriptor) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`SyncDescBuilder`] to the descriptor list
    pub fn push_sync(self, desc_bld: SyncDescriptor) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`ImmediateDescBuilder`] to the descriptor list
    pub fn push_immediate(self, desc_bld: ImmediateDescriptor) -> Result<Self, DmaError> {
        self.push(desc_bld)
    }

    /// Generic method to push any of the descriptor builders wrapped by [`ListDescriptor`]
    pub fn push<T>(mut self, desc_bld: T) -> Result<Self, DmaError>
    where
        T: Into<ListDescriptor> + Copy,
    {
        let desc = self
            .descriptors
            .next()
            .ok_or(DmaError::DescriptorListOverflow)?;

        // Drop any previous descriptor
        let _ = self.prev.take();

        *desc = match desc_bld.into() {
            ListDescriptor::Transfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::LoopTransfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Sync(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Immediate(desc_bld) => desc_bld.into_inner(),
        };

        Ok(Self {
            prev: None,
            descriptors: self.descriptors,
            link_descriptor: self.link_descriptor,
            storage_addr: self.storage_addr,
        })
    }

    /// Convenience method to [`Self::push()`] [`TransferDescBuilder`] to the descriptor list
    pub fn push_linked_transfer(self, desc_bld: TransferDescriptor) -> Result<Self, DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`LoopTransferDescBuilder`] to the descriptor list
    pub fn push_linked_loop(self, desc_bld: LoopTransferDescriptor) -> Result<Self, DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`SyncDescBuilder`] to the descriptor list
    pub fn push_linked_sync(self, desc_bld: SyncDescriptor) -> Result<Self, DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`ImmediateDescBuilder`] to the descriptor list
    pub fn push_linked_immediate(self, desc_bld: ImmediateDescriptor) -> Result<Self, DmaError> {
        self.push_linked(desc_bld)
    }

    /// Generic method to push any of the descriptor builders wrapped by [`ListDescriptor`]
    pub fn push_linked<T>(mut self, desc_bld: T) -> Result<Self, DmaError>
    where
        T: Into<ListDescriptor> + Copy,
    {
        let desc = self
            .descriptors
            .next()
            .ok_or(DmaError::DescriptorListOverflow)?;

        self.link_prev();

        *desc = match desc_bld.into() {
            ListDescriptor::Transfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::LoopTransfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Sync(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Immediate(desc_bld) => desc_bld.into_inner(),
        };
        Ok(Self {
            prev: Some((desc_bld.into(), desc)),
            descriptors: self.descriptors,
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
                ListDescriptor::Transfer(transfer_desc_builder) => transfer_desc_builder
                    .with_link(Addr::Relative(1), true)
                    .into_inner(),
                ListDescriptor::LoopTransfer(loop_transfer_desc_builder) => {
                    loop_transfer_desc_builder.with_link(true).into_inner()
                }
                ListDescriptor::Sync(sync_desc_builder) => sync_desc_builder
                    .with_link(Addr::Relative(1), true)
                    .into_inner(),
                ListDescriptor::Immediate(immediate_desc_builder) => immediate_desc_builder
                    .with_link(Addr::Relative(1), true)
                    .into_inner(),
            };
        }
    }
}

/// Wrapper for all Descriptor Builders which can be pushed to a [`DescList`]
enum ListDescriptor {
    /// [`TransferDescBuilder`]
    Transfer(TransferDescriptor),
    /// [`LoopTransferDescBuilder`]
    LoopTransfer(LoopTransferDescriptor),
    /// [`SyncDescBuilder`]
    Sync(SyncDescriptor),
    /// [`ImmediateDescBuilder`]
    Immediate(ImmediateDescriptor),
}

impl From<TransferDescriptor> for ListDescriptor {
    fn from(value: TransferDescriptor) -> Self {
        Self::Transfer(value)
    }
}

impl From<LoopTransferDescriptor> for ListDescriptor {
    fn from(value: LoopTransferDescriptor) -> Self {
        Self::LoopTransfer(value)
    }
}

impl From<SyncDescriptor> for ListDescriptor {
    fn from(value: SyncDescriptor) -> Self {
        Self::Sync(value)
    }
}

impl From<ImmediateDescriptor> for ListDescriptor {
    fn from(value: ImmediateDescriptor) -> Self {
        Self::Immediate(value)
    }
}
