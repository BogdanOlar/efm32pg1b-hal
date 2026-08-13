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
        self.prev = Some(desc_bld.into());

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

        self.push(desc_bld)
    }

    pub fn into_transfer_descriptor(
        mut self,
        mut transfer_descriptor: TransferDescriptor,
    ) -> TransferDescriptor {
        if self.set_done_ifs().is_ok() {
            transfer_descriptor = transfer_descriptor
                .with_link(Addr::Absolute(self.descriptors.as_ptr().addr()), true);
        } else {
            transfer_descriptor = transfer_descriptor
                .with_done_ifs(true)
                .with_link(Addr::Absolute(0), false);
        }

        transfer_descriptor
    }

    pub fn try_into_link_descriptor(mut self) -> Result<TransferDescriptor, DmaError> {
        self.set_done_ifs()?;
        let mut descr = Descriptor::const_default();
        descr.struct_type_set(StructType::Transfer);

        descr.link_mode_set(AddrMode::Absolute);
        descr.link_addr_set(self.descriptors.as_ptr().addr() >> 2);

        // Setting the Link flag on a Link transfer descriptor will have the effect of the linked list *NOT* being
        // loaded when the Link flag is set on the DMA channel ( [`DmaChannel::link_load()`] )
        descr.link_set(self.index == 0);

        Ok(TransferDescriptor { descr })
    }

    /// Number of descriptors in the list
    pub fn len(&self) -> usize {
        self.index
    }

    /// Check if the descriptor list is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn set_done_ifs(&mut self) -> Result<(), DmaError> {
        if let Some(prev_list_desc) = self.prev.take() {
            if let Some(prev_desc) = self.descriptors.get_mut(self.index - 1) {
                // enable the done interrupt flag on the last descriptor in the list
                *prev_desc = match prev_list_desc {
                    ListDescriptor::Transfer(transfer_descriptor) => {
                        transfer_descriptor.with_done_ifs(true).into()
                    }
                    ListDescriptor::LoopTransfer(loop_transfer_descriptor) => {
                        // TODO: This could be surprising for the user, since the done ISR will wire on each loop completion
                        //       Either disallow this, or make it clear in the documentation
                        loop_transfer_descriptor.with_loop_done_ifs(true).into()
                    }
                    ListDescriptor::Sync(sync_descriptor) => {
                        sync_descriptor.with_done_ifs(true).into()
                    }
                    ListDescriptor::Immediate(immediate_descriptor) => {
                        immediate_descriptor.with_done_ifs(true).into()
                    }
                };

                return Ok(());
            } else {
                // `self.prev` was determined to be `Some` by the parent `if`, so we should always be able to get a
                // valid reference to the corresponding location in `self.descriptors`
                unreachable!();
            }
        }

        // List is empty
        Err(DmaError::InvalidDescriptorList)
    }
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
