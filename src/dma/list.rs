//! Descriptor list and the Descriptor types which can be used with it
//!

use crate::dma::{
    descriptor::{
        Addr, AddrInc, AddrMode, Descriptor, ImmediateDescriptor, LoopTransferDescriptor,
        StructType, SyncDescriptor, TransferCount, TransferDescriptor, UnitSize,
    },
    ChannelId, DmaError,
};
use cortex_m::asm;

/// Descriptor list
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

    /// Convenience method to [`DescList::push()`] [`TransferDescriptor`] to the descriptor list
    pub fn push_transfer(&mut self, desc_bld: TransferDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`DescList::push()`] [`LoopTransferDescriptor`] to the descriptor list
    pub fn push_loop(&mut self, desc_bld: LoopTransferDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`DescList::push()`] [`SyncDescriptor`] to the descriptor list
    pub fn push_sync(&mut self, desc_bld: SyncDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Convenience method to [`DescList::push()`] [`ImmediateDescriptor`] to the descriptor list
    pub fn push_immediate(&mut self, desc_bld: ImmediateDescriptor) -> Result<(), DmaError> {
        self.push(desc_bld)
    }

    /// Add the given descriptor to the list
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

    /// Convenience method to [`DescList::push_linked()`] [`TransferDescriptor`] to the descriptor list
    pub fn push_transfer_linked(&mut self, desc_bld: TransferDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`DescList::push_linked()`] [`LoopTransferDescriptor`] to the descriptor list
    pub fn push_loop_linked(&mut self, desc_bld: LoopTransferDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`DescList::push_linked()`] [`SyncDescriptor`] to the descriptor list
    pub fn push_sync_linked(&mut self, desc_bld: SyncDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Convenience method to [`DescList::push_linked()`] [`ImmediateDescriptor`] to the descriptor list
    pub fn push_immediate_linked(&mut self, desc_bld: ImmediateDescriptor) -> Result<(), DmaError> {
        self.push_linked(desc_bld)
    }

    /// Add the given descriptor to the list, and link it to the previous descriptor
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

    /// Finalize the descriptor list and link it to the given Transfer descriptor
    ///
    /// NOTE: if the last added descriptor is a `LoopTransferDescriptor` and the finalize mode is `DoneIFS`, then the
    ///       done IRQ will fire on each loop completion. Usually, that's probably not what you want, so maybe add
    ///       another non-loop transfer descriptor to trigger the DONE interrupt
    pub fn into_transfer_descriptor(
        mut self,
        mut transfer_descriptor: TransferDescriptor,
        mode: FinMode,
    ) -> TransferDescriptor {
        if self.finalize(mode).is_ok() {
            transfer_descriptor = transfer_descriptor
                .with_link(Addr::Absolute(self.descriptors.as_ptr().addr()), true);
        } else {
            transfer_descriptor = transfer_descriptor
                .with_done_ifs(true)
                .with_link(Addr::Absolute(0), false);
        }

        transfer_descriptor
    }

    /// Finalize the descriptor list and return a Transfer Link descriptor which can be written to the DMA Channel regs
    ///
    /// NOTE: if the last added descriptor is a [`LoopTransferDescriptor`] and the finalize mode is [`FinMode::DoneIFS`]
    ///       , then the done IRQ will fire on each loop completion. Usually, that's probably not what you want, so
    ///       maybe add another non-loop transfer descriptor to trigger the DONE interrupt
    ///
    /// # Errors
    ///
    /// This returns a `DmaError::InvalidDescriptorList` if the list is empty. Makes no sense to use a Link Descriptor
    /// which links to an empty list.
    pub fn try_into_link_descriptor(
        mut self,
        mode: FinMode,
    ) -> Result<TransferDescriptor, DmaError> {
        self.finalize(mode)?;
        let mut descr = Descriptor::const_default();
        descr.struct_type_set(StructType::Transfer);
        descr.link_mode_set(AddrMode::Absolute);
        descr.link_addr_set(self.descriptors.as_ptr().addr() >> 2);
        descr.link_set(true);

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

    /// Set the Done Interrupt-Flag-Set on the last descriptor in the list
    ///
    /// # Errors
    ///
    /// If the list is empty, `DmaError::InvalidDescriptorList` is returned
    fn finalize(&mut self, mode: FinMode) -> Result<(), DmaError> {
        if let Some(prev_list_desc) = self.prev.take() {
            if let Some(prev_desc) = self.descriptors.get_mut(self.index - 1) {
                match mode {
                    FinMode::None => {}
                    FinMode::DoneIFS => {
                        *prev_desc = match prev_list_desc {
                            ListDescriptor::Transfer(transfer_descriptor) => {
                                transfer_descriptor.with_done_ifs(true).into()
                            }
                            ListDescriptor::LoopTransfer(loop_transfer_descriptor) => {
                                loop_transfer_descriptor.with_loop_done_ifs(true).into()
                            }
                            ListDescriptor::Sync(sync_descriptor) => {
                                sync_descriptor.with_done_ifs(true).into()
                            }
                            ListDescriptor::Immediate(immediate_descriptor) => {
                                immediate_descriptor.with_done_ifs(true).into()
                            }
                        };

                        asm::dsb();
                    }
                }

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

/// Wrapper for all Descriptor types which can be pushed to a [`DescList`]
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ListDescriptor {
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

/// Descriptor list finalize mode
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FinMode {
    /// No action
    None,
    /// Add the done interrupt flag set to the last descriptor in the list
    DoneIFS,
}

/// Target address for helper functions `reduced()` and `extended()`
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum TargetAddr {
    /// Use given absolute address and don't increment it
    Fixed(usize),
    /// Use given absolute address and increment it by 1 unit on each copy
    IncrementOne(usize),
}

/// Construct the `TransferDescriptor` for a given number of DMA units. The returned descriptor is meant to be written
/// directly to the DMA Channel registers.
///
/// If necessary, we'll use the DMA Channel loop counter to repeatedly execute a `LoopTransferDescriptor` and a separate
/// `TransferDescriptor` to end the DMA copy and set the Interrupt Done Flag. This is "reduced" because the linked
/// descriptor list does not need an `ImmediateDescriptor` to write the loop count since the counter is written directly
/// to the DMA Channel at the beginning of the DMA Transfer
///
/// **DMA Channel registers**:
///     - `TransferDescriptor`
///     - loop count written to the DMA Channel `LDMA_CHx_LOOP` register if necessary
///
/// **Descriptor list**:
///     - `LoopTransferDescriptor` -> `TransferDescriptor`
pub(crate) fn reduced(
    dma_ch_id: ChannelId,
    src: TargetAddr,
    dst: TargetAddr,
    unit: UnitSize,
    unit_count: usize,
    desc_list: &mut DescList,
) -> Result<TransferDescriptor, DmaError> {
    const NON_LOOP_TRANSFER_COUNT: usize = 2;
    const MAX_LOOP_COUNT: usize = u8::MAX as usize;
    const MAX_TRANSFER_COUNT: usize =
        MAX_LOOP_COUNT * Descriptor::MAX_TRANSFER_UNITS + NON_LOOP_TRANSFER_COUNT;

    let transfer_count = unit_count.div_ceil(Descriptor::MAX_TRANSFER_UNITS);

    if transfer_count > MAX_TRANSFER_COUNT {
        return Err(DmaError::InvalidTransferSize);
    }

    let (src_addr, src_addr_inc) = match src {
        TargetAddr::Fixed(addr) => (addr, AddrInc::None),
        TargetAddr::IncrementOne(addr) => (addr, AddrInc::One),
    };

    let (dst_addr, dst_addr_inc) = match dst {
        TargetAddr::Fixed(addr) => (addr, AddrInc::None),
        TargetAddr::IncrementOne(addr) => (addr, AddrInc::One),
    };

    if transfer_count > 1 {
        let loop_count = transfer_count.saturating_sub(NON_LOOP_TRANSFER_COUNT);

        if loop_count > 0 {
            // Write DMA channel loop count
            crate::dma::mmio::dma()
                .ch(dma_ch_id as usize)
                .loop_()
                .write(|w| unsafe { w.loopcnt().bits((loop_count - 1) as u8) });

            desc_list.push_linked(
                LoopTransferDescriptor::new(
                    Addr::Relative(0),
                    Addr::Relative(0),
                    TransferCount::MAX,
                    Addr::Relative(0),
                    unit,
                )
                .with_src_inc(src_addr_inc)
                .with_dst_inc(dst_addr_inc),
            )?;
        }

        desc_list.push_linked(
            TransferDescriptor::new(
                Addr::Relative(0),
                Addr::Relative(0),
                TransferCount::MAX,
                unit,
            )
            .with_src_inc(src_addr_inc)
            .with_dst_inc(dst_addr_inc),
        )?;
    }

    Ok(TransferDescriptor::new(
        Addr::Absolute(src_addr),
        Addr::Absolute(dst_addr),
        // As a happy coincidence, calling `TransferCount::try_into` with a `unit_count` of 0 will result in an
        // error, so we can use that to set the unit count to `TransferCount::MAX` instead of doing
        // ```
        //  if (unit_count % Descriptor::MAX_TRANSFER_UNITS) == 0 {
        //      Descriptor::MAX_TRANSFER_UNITS
        //  } else {
        //      unit_count % Descriptor::MAX_TRANSFER_UNITS
        //  }
        // ```
        (unit_count % Descriptor::MAX_TRANSFER_UNITS)
            .try_into()
            .unwrap_or(TransferCount::MAX),
        unit,
    )
    .with_src_inc(src_addr_inc)
    .with_dst_inc(dst_addr_inc))
}

/// Construct an "extended" descriptor list and return a Transfer LINK Descriptor suitable to be written to the DMA
/// Channel in order to trigger the transfer with [`crate::dma::DmaChannel::link_load()`].
///
/// This is "extended" because it can use an `ImmediateDescriptor` to write the loop count value to the DMA Channel if
/// necessary.
///
/// **DMA Channel registers**:
///     - LINK `TransferDescriptor`
///
/// **Descriptor list**:
///     - `ImmediateDescriptor` (write to `LDMA_CHx_LOOP`) -> `TransferDescriptor` -> `LoopTransferDescriptor`
///       -> `TransferDescriptor`
pub(crate) fn extended(
    dma_ch_id: ChannelId,
    src: TargetAddr,
    dst: TargetAddr,
    unit: UnitSize,
    unit_count: usize,
    desc_list: &mut DescList,
) -> Result<(), DmaError> {
    const NON_LOOP_TRANSFER_COUNT: usize = 2;
    const MAX_TRANSFER_COUNT: usize =
        u8::MAX as usize * Descriptor::MAX_TRANSFER_UNITS + NON_LOOP_TRANSFER_COUNT;

    let transfer_count = unit_count.div_ceil(Descriptor::MAX_TRANSFER_UNITS);

    if transfer_count > MAX_TRANSFER_COUNT {
        return Err(DmaError::InvalidTransferSize);
    }

    let (src_addr, src_addr_inc) = match src {
        TargetAddr::Fixed(addr) => (addr, AddrInc::None),
        TargetAddr::IncrementOne(addr) => (addr, AddrInc::One),
    };

    let (dst_addr, dst_addr_inc) = match dst {
        TargetAddr::Fixed(addr) => (addr, AddrInc::None),
        TargetAddr::IncrementOne(addr) => (addr, AddrInc::One),
    };

    let loop_count = transfer_count.saturating_sub(NON_LOOP_TRANSFER_COUNT);

    // Immediate Transfer needs to be written _before_ the first Transfer because it will change the SRC and DST
    // registers of the DMA Channel.
    // This way the first Transfer will set the absolute address of the buffer, and the subsequent Transfers can use
    // relative addressing. This is particularly important if the second Transfer is a Loop descriptor which can't use
    // an absolute address
    if loop_count > 0 {
        desc_list.push_linked(ImmediateDescriptor::new(
            (loop_count - 1) as u32,
            crate::dma::mmio::dma()
                .ch(dma_ch_id as usize)
                .loop_()
                .as_ptr()
                .addr(),
        ))?;
    }

    desc_list.push_linked(
        TransferDescriptor::new(
            Addr::Absolute(src_addr),
            Addr::Absolute(dst_addr),
            // As a happy coincidence, calling `TransferCount::try_into` with a `unit_count` of `0` will result in an
            // error (DMA transfer count may not be 0), so we can use that to set the unit count to `TransferCount::MAX`
            // instead of doing
            // ```
            //  if (unit_count % Descriptor::MAX_TRANSFER_UNITS) == 0 {
            //      Descriptor::MAX_TRANSFER_UNITS
            //  } else {
            //      unit_count % Descriptor::MAX_TRANSFER_UNITS
            //  }
            // ```
            (unit_count % Descriptor::MAX_TRANSFER_UNITS)
                .try_into()
                .unwrap_or(TransferCount::MAX),
            unit,
        )
        .with_src_inc(src_addr_inc)
        .with_dst_inc(dst_addr_inc),
    )?;

    if transfer_count > 1 {
        if loop_count > 0 {
            desc_list.push_linked(
                LoopTransferDescriptor::new(
                    Addr::Relative(0),
                    Addr::Relative(0),
                    TransferCount::MAX,
                    Addr::Relative(0),
                    unit,
                )
                .with_src_inc(src_addr_inc)
                .with_dst_inc(dst_addr_inc),
            )?;
        }

        desc_list.push_linked(
            TransferDescriptor::new(
                Addr::Relative(0),
                Addr::Relative(0),
                TransferCount::MAX,
                unit,
            )
            .with_src_inc(src_addr_inc)
            .with_dst_inc(dst_addr_inc),
        )?;
    }

    Ok(())
}
