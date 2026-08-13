//! Descriptor list and the Descriptor types which can be used with it
//!

use crate::dma::{
    descriptor::{
        Addr, AddrMode, Descriptor, ImmediateDescriptor, LoopTransferDescriptor, StructType,
        SyncDescriptor, TransferDescriptor,
    },
    DmaError,
};

/// Descriptor list
pub struct DescList<'a> {
    prev: Option<ListDescriptor>,
    descriptors: &'a mut [Descriptor],
    index: usize,
}

impl<'a> DescList<'a> {
    /// Create a new Descriptor List using the given `descriptors` storage
    /// This constructor will reset `descriptors` to default
    pub fn new(descriptors: &'a mut [Descriptor]) -> Self {
        descriptors.fill(Descriptor::default());
        Self {
            prev: None,
            descriptors,
            index: usize::default(),
        }
    }

    /// Convenience method to [`Self::push()`] [`TransferDescBuilder`] to the descriptor list
    pub fn push_transfer(&mut self, desc_bld: TransferDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`LoopTransferDescBuilder`] to the descriptor list
    pub fn push_loop(&mut self, desc_bld: LoopTransferDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`SyncDescBuilder`] to the descriptor list
    pub fn push_sync(&mut self, desc_bld: SyncDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`ImmediateDescBuilder`] to the descriptor list
    pub fn push_immediate(&mut self, desc_bld: ImmediateDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    pub fn push<T>(&mut self, desc_bld: T) -> Result<(), DmaError>
    where
        T: Into<ListDescriptor> + Copy,
    {
        let desc = self
            .descriptors
            .get_mut(self.index)
            .ok_or(DmaError::DescriptorListOverflow)?;

        *desc = match desc_bld.into() {
            ListDescriptor::Transfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::LoopTransfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Sync(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Immediate(desc_bld) => desc_bld.into_inner(),
        };

        self.index += 1;
        self.prev = None;

        Ok(())
    }

    /// Convenience method to [`Self::push()`] [`TransferDescBuilder`] to the descriptor list
    pub fn push_transfer_linked(&mut self, desc_bld: TransferDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`LoopTransferDescBuilder`] to the descriptor list
    pub fn push_loop_linked(&mut self, desc_bld: LoopTransferDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`SyncDescBuilder`] to the descriptor list
    pub fn push_sync_linked(&mut self, desc_bld: SyncDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`Self::push()`] [`ImmediateDescBuilder`] to the descriptor list
    pub fn push_immediate_linked(&mut self, desc_bld: ImmediateDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    pub fn push_linked<T>(&mut self, desc_bld: T) -> Result<(), DmaError>
    where
        T: Into<ListDescriptor> + Copy,
    {
        let desc = self
            .descriptors
            .get_mut(self.index)
            .ok_or(DmaError::DescriptorListOverflow)?;

        *desc = match desc_bld.into() {
            ListDescriptor::Transfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::LoopTransfer(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Sync(desc_bld) => desc_bld.into_inner(),
            ListDescriptor::Immediate(desc_bld) => desc_bld.into_inner(),
        };

        // Modify the previous descriptor (if it exists) to link to the current descriptor in the list
        if let Some(prev_list_desc) = self.prev.take() {
            if let Some(prev_desc) = self.descriptors.get_mut(self.index - 1) {
                *prev_desc = match prev_list_desc {
                    ListDescriptor::Transfer(transfer_desc) => transfer_desc
                        .with_link(Addr::Relative(1), true)
                        .into_inner(),
                    ListDescriptor::LoopTransfer(loop_transfer_desc) => {
                        loop_transfer_desc.with_link(true).into_inner()
                    }
                    ListDescriptor::Sync(sync_desc) => {
                        sync_desc.with_link(Addr::Relative(1), true).into_inner()
                    }
                    ListDescriptor::Immediate(immediate_desc) => immediate_desc
                        .with_link(Addr::Relative(1), true)
                        .into_inner(),
                };
            }
        }

        self.index += 1;
        self.prev = Some(desc_bld.into());

        Ok(())
    }

    pub fn finalize_with_transfer_descriptor(
        self,
        mut transfer_descriptor: TransferDescriptor,
    ) -> FinalizedList {
        transfer_descriptor = transfer_descriptor.with_link(
            Addr::Absolute(self.storage_addr()),
            // Only link if this list has some linked descriptors
            self.index > 0,
        );

        FinalizedList {
            size: self.index,
            transfer_descriptor,
        }
    }

    pub fn finalize_with_link_descriptor(self) -> FinalizedList {
        let mut descr = Descriptor::const_default();
        descr.struct_type_set(StructType::Transfer);

        descr.link_mode_set(AddrMode::Absolute);
        descr.link_addr_set(self.storage_addr() >> 2);
        // Only link if this list has some linked descriptors
        // Setting the Link flag on a Link transfer descriptor will have the effect of the linked list *NOT* being
        // loaded when the Link flag is set on the DMA channel ( [`DmaChannel::link_load()`] )
        descr.link_set(self.index == 0);

        FinalizedList {
            size: self.index,
            transfer_descriptor: TransferDescriptor { descr },
        }
    }

    pub(crate) fn storage_addr(&self) -> usize {
        self.descriptors.as_ptr().addr()
    }
}

pub struct FinalizedList {
    pub size: usize,
    pub transfer_descriptor: TransferDescriptor,
}

/// Wrapper for all Descriptor Builders which can be pushed to a [`DescList`]
enum ListDescriptor {
    /// [`TransferDescriptor`]
    Transfer(TransferDescriptor),
    /// [`LoopTransferDescriptor`]
    LoopTransfer(LoopTransferDescriptor),
    /// [`SyncDescriptor`]
    Sync(SyncDescriptor),
    /// [`ImmediateDescriptor`]
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
