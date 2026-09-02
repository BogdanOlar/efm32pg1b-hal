//! Embassy support dor DMA
//!

use crate::dma::{descriptor::TransferDescriptor, irq, DmaChannel, DmaResult, CHANNEL_COUNT};
#[cfg(feature = "debug-spi-dma-defmt-info")]
use defmt::info;
use embassy_sync::waitqueue::AtomicWaker;

/// Embassy task wakers for each DMA channel
static DMA_WAKERS: [AtomicWaker; CHANNEL_COUNT] = [const { AtomicWaker::new() }; _];

impl DmaChannel {
    /// Perform an async DMA transfer with the given Descriptor.
    ///
    /// If `with_sw_trigger` is set then the DMA transfer will immediatelly be software-triggered.
    pub async fn transfer_async(
        &mut self,
        desc: &TransferDescriptor,
        with_sw_trigger: bool,
    ) -> DmaResult {
        self.cancel();

        // Set the async IRQ handler
        critical_section::with(|cs| {
            irq::set_handler(cs, self.id(), |id, transfer_result| {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("IRQ {}: {}", id, transfer_result);
                // signal to the main thread that transfer is resolved
                critical_section::with(|cs_inner| {
                    irq::irq_ch_set(cs_inner, id, Some(transfer_result))
                });
                // Wake the task awaiting on this transfer
                DMA_WAKERS[id as usize].wake();
            })
        });

        unsafe { self.start(desc, with_sw_trigger) };

        TransferFuture::new(self).await
    }
}

struct TransferFuture<'a> {
    ch: &'a mut DmaChannel,
}

impl<'a> TransferFuture<'a> {
    fn new(ch: &'a mut DmaChannel) -> Self {
        Self { ch }
    }
}

impl<'a> core::future::Future for TransferFuture<'a> {
    type Output = DmaResult;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        DMA_WAKERS[self.ch.id() as usize].register(cx.waker());

        match self.ch.try_resolve() {
            Some(transfer_result) => core::task::Poll::Ready(transfer_result),
            None => core::task::Poll::Pending,
        }
    }
}

// impl DmaChannel {
//     /// Start an async memory-to-memory DMA transfer
//     pub fn into_async_transfer<'a, W: Sized>(
//         self,
//         src: &'a [W],
//         dst: &'a mut [W],
//     ) -> ChannelTransferFuture<'a, W> {
//         let id = self.id;

//         // Set the IRQ handler for this channel transfer
//         critical_section::with(|cs| {
//             dma::irq::set_handler(cs, id, |id, transfer_result| {
//                 // signal to the main thread that transfer is resolved
//                 critical_section::with(|csd| dma::irq::irq_ch_set(csd, id, Some(transfer_result)));
//                 // Wake the task awaiting on this transfer
//                 DMA_WAKERS[id as usize].wake();
//             })
//         });

//         let mut transfer = ChannelTransfer::new(self, src, dst);
//         transfer.start();
//         ChannelTransferFuture { transfer }
//     }
// }

// /// Async DMA memory-to-memory transfer
// #[must_use = "futures do nothing unless you `.await` or poll them"]
// pub struct ChannelTransferFuture<'a, W: Sized> {
//     transfer: ChannelTransfer<'a, W>,
// }

// impl<'a, W: Sized> Future for ChannelTransferFuture<'a, W> {
//     type Output = ChannelTransferResult<'a, W>;

//     fn poll(
//         mut self: core::pin::Pin<&mut Self>,
//         cx: &mut core::task::Context<'_>,
//     ) -> core::task::Poll<Self::Output> {
//         DMA_WAKERS[self.transfer.id() as usize].register(cx.waker());

//         if let Some(transfer_result) = self.transfer.try_resolve() {
//             Poll::Ready(transfer_result)
//         } else {
//             Poll::Pending
//         }
//     }
// }
