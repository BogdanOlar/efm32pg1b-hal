//! DMA interrupt handling

use crate::{
    dma::{mmio, ChannelId, DmaError, DmaResult, CHANNEL_COUNT},
    pac::interrupt,
};
use core::cell::RefCell;
use critical_section::{CriticalSection, Mutex};

/// Handler function for a DMA interrupt
type DmaIrqHandler = fn(ChannelId, DmaResult);

/// Handler which does nothing
const fn default_handler(_: ChannelId, _: DmaResult) {}

/// Communication channels between DMA IRQ and the main thread. One for each `DmaChannel`
static IRQ_CHANNELS: Mutex<RefCell<[Option<DmaResult>; CHANNEL_COUNT]>> =
    Mutex::new(RefCell::new([None; _]));

/// Interrupt handlers for each DMA Channel
static HANDLERS: Mutex<RefCell<[DmaIrqHandler; CHANNEL_COUNT]>> =
    Mutex::new(RefCell::new([default_handler; _]));

pub(crate) fn irq_ch_take(cs: CriticalSection, id: ChannelId) -> Option<DmaResult> {
    IRQ_CHANNELS.borrow(cs).borrow_mut()[id as usize].take()
}

pub(crate) fn irq_ch_set(cs: CriticalSection, id: ChannelId, new: Option<DmaResult>) {
    IRQ_CHANNELS.borrow(cs).borrow_mut()[id as usize] = new;
}

/// Set the handler function for the given DMA channel
pub(crate) fn set_handler(cs: CriticalSection, id: ChannelId, handler: DmaIrqHandler) {
    HANDLERS.borrow(cs).borrow_mut()[id as usize] = handler;
}

/// Clear the handler function for the given DMA channel
pub(crate) fn clear_handler(cs: CriticalSection, id: ChannelId) {
    HANDLERS.borrow(cs).borrow_mut()[id as usize] = default_handler;
}

#[interrupt]
fn LDMA() {
    // process any channel error
    if let Some(id) = mmio::ch_error() {
        mmio::if_error_clear();
        mmio::ifc_set(id);
        let handle = critical_section::with(|cs| HANDLERS.borrow(cs).borrow()[id as usize]);
        handle(id, Err(DmaError::Transfer));
    }

    // process channel done flags
    for id in mmio::if_raised() {
        mmio::ifc_set(id);
        let handle = critical_section::with(|cs| HANDLERS.borrow(cs).borrow()[id as usize]);
        handle(id, Ok(()));
    }
}
