//! Embassy support dor DMA
//!

use crate::dma::{
    descriptor::TransferDescriptor,
    irq,
    transfer::{ChannelTransfer, TransferParams},
    DmaChannel, DmaResult, CHANNEL_COUNT,
};
#[cfg(feature = "debug-spi-dma-defmt-info")]
use defmt::info;
use embassy_sync::waitqueue::AtomicWaker;

/// Embassy task wakers for each DMA channel
static DMA_WAKERS: [AtomicWaker; CHANNEL_COUNT] = [const { AtomicWaker::new() }; _];

impl DmaChannel {
    /// Perform an async DMA transfer with the given Descriptor.
    ///
    /// If `with_sw_trigger` is set then the DMA transfer will immediatelly be software-triggered.
    pub(crate) async fn transfer_async<'tl, P: TransferParams<'tl>>(
        &'tl mut self,
        desc: &TransferDescriptor,
        with_sw_trigger: bool,
        params: P,
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

        TransferFuture::new(ChannelTransfer::new(self, params)).await
    }
}

struct TransferFuture<'tl, P: TransferParams<'tl>> {
    ch_transfer: ChannelTransfer<'tl, P>,
}

impl<'tl, P: TransferParams<'tl>> TransferFuture<'tl, P> {
    fn new(ch_transfer: ChannelTransfer<'tl, P>) -> Self {
        Self { ch_transfer }
    }
}

impl<'tl, P: TransferParams<'tl>> core::future::Future for TransferFuture<'tl, P> {
    type Output = DmaResult;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        DMA_WAKERS[self.ch_transfer.ch.id() as usize].register(cx.waker());

        match self.ch_transfer.try_resolve() {
            Some(transfer_result) => core::task::Poll::Ready(transfer_result),
            None => core::task::Poll::Pending,
        }
    }
}
