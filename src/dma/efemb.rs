//! Embassy support dor DMA
//!

use crate::dma::{self, ChannelTransfer, ChannelTransferResult, DmaChannel};
use core::{cmp::min, future::Future, task::Poll};
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
        ChannelTransferFuture::new(
            self,
            src,
            dst,
            min(core::mem::size_of_val(src), core::mem::size_of_val(dst)),
        )
    }
}

/// Async DMA memory-to-memory transfer
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ChannelTransferFuture<'a, W: Sized> {
    inner: ChannelTransfer<'a, W>,
}

impl<'a, W: Sized> ChannelTransferFuture<'a, W> {
    fn new(ch: DmaChannel, src: &'a [W], dst: &'a mut [W], byte_count: usize) -> Self {
        critical_section::with(|cs| {
            // Set the IRQ handler for this channel to wake the task on interrupt
            // Will get cleared at the end of the transfer
            dma::irq::set_handler(cs, ch.id(), |id, _| {
                DMA_WAKERS[id as usize].wake();
            });
        });

        Self {
            inner: ChannelTransfer::new(ch, src, dst, byte_count),
        }
    }
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
