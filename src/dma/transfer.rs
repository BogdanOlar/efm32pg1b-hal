//! DMA Channel transfer
//!

use cortex_m::asm;

use crate::dma::{
    descriptor::{self, Addr, Descriptor, TransferDescriptor, UnitSize},
    irq, mmio, ChannelId, DmaChannel, DmaResult,
};

/// DMA channel specialised for memory-to-memory transfer
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MemoryTransfer<'a, Word: Copy + 'static> {
    /// DMA Channel transfer parameters.
    /// The `Option` is needed because this type implements `Drop`, and we may need to release the params before this
    /// struct is dropped
    params: Option<MemoryTransferParams<'a, Word>>,
    id: ChannelId,
    unit: UnitSize,
}

impl<'a, Word: Copy + 'static> MemoryTransfer<'a, Word> {
    pub(crate) fn new(ch: &'a mut DmaChannel, src: &'a [Word], dst: &'a mut [Word]) -> Self {
        // Decide which unit/type the transfer may use
        let unit = if src.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && src.len().is_multiple_of(size_of::<u32>())
            && dst.len().is_multiple_of(size_of::<u32>())
        {
            UnitSize::Word
        } else if src.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && src.len().is_multiple_of(size_of::<u16>())
            && dst.len().is_multiple_of(size_of::<u16>())
        {
            UnitSize::Halfword
        } else {
            UnitSize::Byte
        };

        let id = ch.id();
        let byte_count = core::mem::size_of_val(src);

        // Handle 0 sized transfers: set a dummy success token and skip hardware setup
        if byte_count == 0 {
            critical_section::with(|cs| irq::irq_ch_set(cs, id, Some(Ok(()))));
            return Self {
                params: Some(MemoryTransferParams { ch, src, dst }),
                id,
                unit,
            };
        }

        // Reset channel state
        ch.set_ien(false);
        ch.clear_interrupt_flags();
        ch.set_enabled(false);
        ch.set_done(false);

        critical_section::with(|cs| {
            let _ = irq::irq_ch_take(cs, id);
        });

        let dst_bytes: &mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(dst.as_ptr() as *mut u8, byte_count) };

        assert_eq!(byte_count, core::mem::size_of_val(dst_bytes));
        assert_ne!(dst_bytes.len(), 0);

        let unit_byte_size = 1 << unit as u8;
        let total_units = dst_bytes.len() / unit_byte_size;
        assert_eq!(dst_bytes.len() % unit_byte_size, 0);

        let arr_end = dst_bytes[dst_bytes.len()..].as_ptr().addr();
        let aligned_end_addr = arr_end - (arr_end % align_of::<Descriptor>());

        let last_descr_addr = aligned_end_addr - size_of::<Descriptor>();
        let last_chunk_min_units = (arr_end - last_descr_addr) / unit_byte_size;
        assert_eq!((arr_end - last_descr_addr) % unit_byte_size, 0);

        let descr_count = total_units.div_ceil(Descriptor::MAX_TRANSFER_UNITS);

        // First descriptor will be written to DMA channel register, not to the descriptor list
        let linked_list_count = descr_count - 1;
        let linked_list_start_addr =
            aligned_end_addr - (linked_list_count * size_of::<Descriptor>());
        // Create the descriptor list at the end of the destination buffer
        let descriptor_list = unsafe {
            core::slice::from_raw_parts_mut(
                linked_list_start_addr as *mut Descriptor,
                linked_list_count,
            )
        };

        let first_descriptor = build_m2m_descriptors(
            src.as_ptr().addr(),
            dst.as_ptr().addr(),
            total_units,
            last_chunk_min_units,
            descriptor_list,
            unit,
        );

        // Start the transfer. The descriptor was built safely by `build_descriptors`.
        unsafe { ch.start(&first_descriptor, true) };

        Self {
            params: Some(MemoryTransferParams { ch, src, dst }),
            id,
            unit,
        }
    }

    /// Get the DMA Channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Try to complete the transfer.
    ///
    /// If the transfer has completed then the transfer is disabled and the transfer result is returned.
    /// Will only return `Some` once, when the transfer is complete.
    ///
    /// Example:
    ///
    /// ```rust,no_run
    ///     // start the transfer
    ///     let mut transfer = ch.memory_transfer(src, dst)?;
    ///
    ///     // wait for transfer to complete
    ///     let transfer_result = loop {
    ///         match transfer.try_resolve() {
    ///             Some(res) => break res,
    ///             None => {
    ///                 // transfer still in progress
    ///             }
    ///         }
    ///     };
    ///
    ///     // `try_resolve()` should only return `Some` _once_ (in the loop above)
    ///     assert!(transfer.try_resolve().is_none());
    /// ```
    pub fn try_resolve(&mut self) -> Option<DmaResult> {
        if let Some(transfer_result) = critical_section::with(|cs| irq::irq_ch_take(cs, self.id)) {
            if self.params.take().is_some() {
                // Disable channel
                mmio::ien_clear(self.id);
                mmio::ifc_set(self.id);
                mmio::chen_clear(self.id);
                mmio::ch_done_clear(self.id);
                // Clear DMA channel handler
                critical_section::with(|cs| irq::clear_handler(cs, self.id));

                Some(transfer_result)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Cancel the memory transfer
    pub fn cancel(&mut self) {
        if let Some(p) = self.params.take() {
            p.ch.cancel();
        }
    }

    /// Get the Unit size that the transfer is using: byte, half-word (16 bits), word (32 bits)
    ///
    /// The unit size is calculated dynamically when the Transfer is created, based on the alignment an length of
    /// `self.src` and `self.dst`, and it favors the widest bitwidth
    pub fn unit(&self) -> UnitSize {
        self.unit
    }
}

impl<'a, Word: Copy + 'static> Drop for MemoryTransfer<'a, Word> {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Parameters used to create a DMA Transfer (both sync and async)
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MemoryTransferParams<'a, Word: Copy + 'static> {
    /// DMA channel
    pub ch: &'a mut DmaChannel,
    /// Source buffer
    pub src: &'a [Word],
    /// Destination buffer
    pub dst: &'a mut [Word],
}

/// Result type of a DMA transfer (both sync and async)
pub type ChannelTransferResult<'a, W> =
    Result<(MemoryTransferParams<'a, W>, usize), MemoryTransferParams<'a, W>>;

/// Build the first transfer descriptor, and any necessary linked descriptors for a memory-to-memory transfer
fn build_m2m_descriptors(
    src_addr: usize,
    dst_addr: usize,
    total_units: usize,
    last_chunk_min_units: usize,
    descriptor_list: &mut [Descriptor],
    unit: UnitSize,
) -> TransferDescriptor {
    let mut remaining_units = total_units;

    // Create first descriptor
    let first_descr_units = if remaining_units > Descriptor::MAX_TRANSFER_UNITS {
        Descriptor::MAX_TRANSFER_UNITS.min(remaining_units - last_chunk_min_units)
    } else {
        remaining_units
    };
    remaining_units -= first_descr_units;

    let first_descriptor = {
        let mut descr_builder = TransferDescriptor::new(
            Addr::Absolute(src_addr),
            Addr::Absolute(dst_addr),
            first_descr_units.try_into().unwrap(),
            unit,
        )
        .with_struct_req(true)
        .with_block_size(descriptor::BlockSize::All);

        if remaining_units > 0 {
            descr_builder =
                descr_builder.with_link(Addr::Absolute(descriptor_list.as_ptr().addr()), true);
        }

        descr_builder
    };

    // Fill in the linked descriptors
    let descriptor_list_count = descriptor_list.len();
    for (i, ser_descr) in descriptor_list.iter_mut().enumerate() {
        let is_last = i == (descriptor_list_count - 1);

        let descr_units = if is_last {
            remaining_units
        } else {
            Descriptor::MAX_TRANSFER_UNITS.min(remaining_units - last_chunk_min_units)
        };
        assert!(descr_units <= Descriptor::MAX_TRANSFER_UNITS);

        let addr_offset = (total_units - remaining_units) * unit.byte_count();

        let mut transfer_descr = TransferDescriptor::new(
            Addr::Absolute(src_addr + addr_offset),
            Addr::Absolute(dst_addr + addr_offset),
            descr_units.try_into().unwrap(),
            unit,
        )
        .with_struct_req(true)
        .with_block_size(descriptor::BlockSize::All);

        if !is_last {
            transfer_descr = transfer_descr.with_link(Addr::Relative(1), true);
        }

        *ser_descr = transfer_descr.into_inner();
        remaining_units -= descr_units;
    }
    assert_eq!(remaining_units, 0);

    // make sure all linked descriptors have been written before proceeding
    asm::dsb();

    first_descriptor
}
