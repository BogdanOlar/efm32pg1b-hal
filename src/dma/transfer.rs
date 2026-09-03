//! DMA Channel transfer
//!

use crate::dma::{irq, ChannelId, DmaChannel, DmaResult};

/// DMA channel transfer
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelTransfer<'tl, P: TransferParams<'tl>> {
    /// DMA channel
    ch: &'tl mut DmaChannel,
    /// DMA Channel transfer parameters.
    params: P,
}

impl<'tl, P: TransferParams<'tl>> ChannelTransfer<'tl, P> {
    pub(crate) fn new(ch: &'tl mut DmaChannel, params: P) -> Self {
        Self { ch, params }
    }

    /// Get the DMA Channel ID
    pub fn id(&self) -> ChannelId {
        self.ch.id()
    }

    /// Try to complete the transfer.
    ///
    /// If the transfer has completed then the transfer is disabled and the transfer result is returned.
    /// Will only return `Some` **ONCE**, when the transfer is complete.
    ///
    pub fn try_resolve(&mut self) -> Option<DmaResult> {
        if let Some(transfer_result) =
            critical_section::with(|cs| irq::irq_ch_take(cs, self.ch.id()))
        {
            self.ch.stop();

            critical_section::with(|cs| {
                // Clear the IRQ handler
                irq::clear_handler(cs, self.ch.id());
            });

            Some(transfer_result)
        } else {
            None
        }
    }

    /// Cancel the memory transfer
    pub fn cancel(&mut self) {
        self.ch.cancel();
    }
}

/// Marker trait for the parameters of a DMA transfer
pub trait TransferParams<'tl> {}

impl<'tl, P: TransferParams<'tl>> Drop for ChannelTransfer<'tl, P> {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Parameters used to create a DMA Transfer (both sync and async)
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MemoryTransferParams<'a, Word: Copy + 'static> {
    /// Source buffer
    pub src: &'a [Word],
    /// Destination buffer
    pub dst: &'a mut [Word],
}

impl<'a, Word: Copy + 'static> TransferParams<'a> for MemoryTransferParams<'a, Word> {}

/// Result type of a DMA transfer (both sync and async)
pub type ChannelTransferResult<'a, W> =
    Result<(MemoryTransferParams<'a, W>, usize), MemoryTransferParams<'a, W>>;
