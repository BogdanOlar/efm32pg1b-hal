//! Embassy support dor DMA
//!

use crate::dma::{self, ChannelTransfer, ChannelTransferResult, DmaChannel};
use core::{future::Future, task::Poll};
use embassy_sync::waitqueue::AtomicWaker;

/// Embassy task wakers for each DMA channel
static DMA_WAKERS: [AtomicWaker; DmaChannel::COUNT] = [const { AtomicWaker::new() }; _];

impl DmaChannel {
    /// Start an async memory-to-memory DMA transfer
    pub fn into_async_transfer<'a, W: Sized>(
        self,
        src: &'a [W],
        dst: &'a mut [W],
    ) -> ChannelTransferFuture<'a, W> {
        let id = self.id;
        let mut transfer = ChannelTransfer::new(self, src, dst);

        // Set the IRQ handler for this channel transfer
        critical_section::with(|cs| {
            dma::irq::set_handler(cs, id, |id, channel_error| {
                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| dma::irq::irq_ch_set(csd, id, Some(channel_error)));
                // Wake the task awaiting on this transfer
                DMA_WAKERS[id as usize].wake();
            })
        });

        transfer.start();

        ChannelTransferFuture { inner: transfer }
    }
}

/// Async DMA memory-to-memory transfer
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ChannelTransferFuture<'a, W: Sized> {
    inner: ChannelTransfer<'a, W>,
}

impl<'a, W: Sized> Future for ChannelTransferFuture<'a, W> {
    type Output = ChannelTransferResult<'a, W>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        DMA_WAKERS[self.inner.id as usize].register(cx.waker());

        if let Some(res) = self.inner.check_done() {
            Poll::Ready(res)
        } else {
            Poll::Pending
        }
    }
}
