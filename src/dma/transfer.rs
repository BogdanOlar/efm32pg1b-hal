//! DMA Channel transfer
//!

use cortex_m::asm;

use crate::dma::{
    descriptor::{self, Addr, Descriptor, TransferDescriptor, UnitSize},
    irq, mmio, ChannelId, DmaChannel,
};

/// DMA channel specialised for memory-to-memory transfer
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelTransfer<'a, W: Sized> {
    /// DMA Channel transfer parameters.
    /// The `Option` is needed because this type implements `Drop`, and we may need to release the params before this
    /// struct is dropped
    params: Option<ChannelTransferParams<'a, W>>,
    id: ChannelId,
    byte_count: usize,
    unit: UnitSize,
}

impl<'a, W: Sized> ChannelTransfer<'a, W> {
    pub(crate) fn new(ch: DmaChannel, src: &'a [W], dst: &'a mut [W]) -> Self {
        let byte_count = core::mem::size_of_val(src).min(core::mem::size_of_val(dst));

        // Decide which unit/type the transfer may use
        let unit = if src.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.len().is_multiple_of(size_of::<u32>())
        {
            UnitSize::Word
        } else if src.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.len().is_multiple_of(size_of::<u16>())
        {
            UnitSize::Halfword
        } else {
            UnitSize::Byte
        };

        let id = ch.id;
        Self {
            params: Some(ChannelTransferParams { ch, src, dst }),
            id,
            byte_count,
            unit,
        }
    }

    /// Get the DMA Channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Start the DMA transfer
    pub(crate) fn start(&mut self) {
        // handle 0 sized transfers
        if self.byte_count == 0 {
            // Set a dummy success token in the IRQ channel for this DMA channel
            critical_section::with(|cs| irq::irq_ch_set(cs, self.id, Some(false)));

            // skip the rest of the init
            return;
        }

        mmio::ien_clear(self.id);
        mmio::ifc_set(self.id);
        mmio::chen_clear(self.id);
        mmio::chdone_clear(self.id);

        critical_section::with(|cs| {
            // Clear any existing content in the IRQ channel of this DMA channel
            irq::irq_ch_take(cs, self.id);
        });

        let dst: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(
                self.params.as_ref().unwrap().dst.as_ptr() as *mut u8,
                self.byte_count,
            )
        };

        assert_eq!(self.byte_count, core::mem::size_of_val(dst));
        assert_ne!(dst.len(), 0);

        let unit_byte_size = 1 << self.unit as u8;
        let total_units = dst.len() / unit_byte_size;
        assert_eq!(dst.len() % unit_byte_size, 0);

        let arr_end = dst[dst.len()..].as_ptr().addr();
        let aligned_end_addr = arr_end - (arr_end % align_of::<Descriptor>());

        let last_descr_addr = aligned_end_addr - size_of::<Descriptor>();
        let last_chunk_min_units = (arr_end - last_descr_addr) / unit_byte_size;
        assert_eq!((arr_end - last_descr_addr) % unit_byte_size, 0);

        let descr_count = if total_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
            total_units / Descriptor::MAX_TRANSFER_UNITS
        } else {
            (total_units / Descriptor::MAX_TRANSFER_UNITS) + 1
        };
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
        let first_descriptor = match self.unit {
            UnitSize::Byte => self.build_descriptors(
                total_units,
                last_chunk_min_units,
                descriptor_list,
                UnitSize::Byte,
            ),
            UnitSize::Halfword => self.build_descriptors(
                total_units,
                last_chunk_min_units,
                descriptor_list,
                UnitSize::Halfword,
            ),
            UnitSize::Word => self.build_descriptors(
                total_units,
                last_chunk_min_units,
                descriptor_list,
                UnitSize::Word,
            ),
        };

        // First descriptor is always written directly to the DMA peripheral in order to support transfers smaller than
        // the size of a descriptor (in which case we don't use descriptor linked list)
        // FIXME: maybe write this to the channel, not the regs directly?
        mmio::ch_write_descriptor(self.id, &first_descriptor.into_inner());

        // start the transfer
        mmio::ien_set(self.id);
        mmio::chen_set(self.id);
        mmio::swreq(self.id);
    }

    /// Check if DMA transfer is done. Will only return `Some` once, when the transfer is complete.
    ///
    /// Example:
    ///
    /// ```rust,no_run
    ///     // start the transfer
    ///     let mut transfer = ch.into_transfer(src, dst);
    ///
    ///     // wait for transfer to complete
    ///     let transfer_result = loop {
    ///         match transfer.check_done() {
    ///             Some(res) => break res,
    ///             None => {
    ///                 info!(".")
    ///             }
    ///         }
    ///     };
    ///
    ///     // `check_done()` should only return `Some` _once_ (in the loop above)
    ///     assert!(transfer.check_done().is_none());
    ///
    ///     // Print results
    ///     match &transfer_result {
    ///         Ok((params, bytes_count)) => {
    ///             info!("Ok: {}, {} bytes", params.ch, bytes_count);
    ///         }
    ///         Err(params) => {
    ///             error!("Err: {}", params.ch);
    ///         }
    ///     }
    /// ```
    pub fn check_done(&mut self) -> Option<ChannelTransferResult<'a, W>> {
        if let Some(ch_error) = critical_section::with(|cs| irq::irq_ch_take(cs, self.id)) {
            if let Some(params) = self.params.take() {
                // Disable channel
                mmio::ien_clear(self.id);
                mmio::ifc_set(self.id);
                mmio::chen_clear(self.id);
                mmio::chdone_clear(self.id);
                // Clear DMA channel handler
                critical_section::with(|cs| irq::clear_handler(cs, self.id));

                match ch_error {
                    true => Some(Err(params)),
                    false => Some(Ok((params, self.byte_count))),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the Unit size that the transfer is using: byte, half-word (16 bits), word (32 bits)
    ///
    /// The unit size is calculated dynamically when the Transfer is created, based on the alignment an length of
    /// `self.src` and `self.dst`, and it favors the widest bitwidth (word--32 bits), followed by half-word, followed
    /// by byte size
    pub fn unit(&self) -> UnitSize {
        self.unit
    }

    /// Build the first transfer descriptor, and any necessary linked descriptors
    fn build_descriptors(
        &mut self,
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
                Addr::Absolute(self.params.as_ref().unwrap().src.as_ptr().addr()),
                Addr::Absolute(self.params.as_ref().unwrap().dst.as_ptr().addr()),
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
                Addr::Absolute(self.params.as_ref().unwrap().src.as_ptr().addr() + addr_offset),
                Addr::Absolute(self.params.as_ref().unwrap().dst.as_ptr().addr() + addr_offset),
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
}

impl<'a, W: Sized> Drop for ChannelTransfer<'a, W> {
    fn drop(&mut self) {
        if mmio::chen(self.id) {
            panic!("`ChannelTransfer` was dropped while DMA channel was still active");
        }
    }
}

/// Parameters used to create a DMA Transfer (both sync and async)
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelTransferParams<'a, W: Sized> {
    /// DMA channel
    pub ch: DmaChannel,
    /// Source buffer
    pub src: &'a [W],
    /// Destination buffer
    pub dst: &'a mut [W],
}

/// Result type of a DMA transfer (both sync and async)
pub type ChannelTransferResult<'a, W> =
    Result<(ChannelTransferParams<'a, W>, usize), ChannelTransferParams<'a, W>>;
