//! Linked Direct Memory Access
//!
//! # ChannelTransfer
//!
//! Memory-to-memory transfer
//!
//! **WARNING**: May panic if the `ChannelTransfer` is dropped while the DMA channel is still active.
//!              Use `ChannelTransfer::check_done()` to determine if the DMA transfer completed.
//!

pub mod descriptor;
pub mod list;
pub(crate) mod mmio;
pub mod transfer;

#[cfg(feature = "efemb")]
pub mod efemb;

use crate::{
    dma::{descriptor::TransferDescriptor, irq::set_handler},
    pac::{Interrupt, Ldma, NVIC},
};

/// Number of DMA channels
const CHANNEL_COUNT: usize = 1 << 3;

/// DMA driver
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dma {
    /// DMA channel 0
    pub ch0: DmaChannel,
    /// DMA channel 1
    pub ch1: DmaChannel,
    /// DMA channel 2
    pub ch2: DmaChannel,
    /// DMA channel 3
    pub ch3: DmaChannel,
    /// DMA channel 4
    pub ch4: DmaChannel,
    /// DMA channel 5
    pub ch5: DmaChannel,
    /// DMA channel 6
    pub ch6: DmaChannel,
    /// DMA channel 7
    pub ch7: DmaChannel,
}

impl Dma {
    /// Initialize DMA
    pub fn init(_dma_p: Ldma) -> Self {
        // Enable DMA clock
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfbusclken0().modify(|_, w| w.ldma().set_bit());

        unsafe {
            NVIC::unmask(Interrupt::LDMA);
        }

        Self {
            ch0: DmaChannel { id: ChannelId::Ch0 },
            ch1: DmaChannel { id: ChannelId::Ch1 },
            ch2: DmaChannel { id: ChannelId::Ch2 },
            ch3: DmaChannel { id: ChannelId::Ch3 },
            ch4: DmaChannel { id: ChannelId::Ch4 },
            ch5: DmaChannel { id: ChannelId::Ch5 },
            ch6: DmaChannel { id: ChannelId::Ch6 },
            ch7: DmaChannel { id: ChannelId::Ch7 },
        }
    }
}

/// DMA channel singleton
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DmaChannel {
    /// Channel ID
    id: ChannelId,
}

impl DmaChannel {
    /// Reset channel to a known state
    pub fn reset(&mut self) {
        // Cancel any on-going transfer
        self.cancel();

        mmio::ctrl_syncprsseten_clear(self.id);
        mmio::ctrl_syncprsclren_clear(self.id);
        mmio::sync_clear(self.id);
        mmio::dbghalt_clear(self.id);
        mmio::reqdis_clear(self.id);
        mmio::reqclear_set(self.id);
        mmio::set_reqsel(self.id, ChReqSel::None);

        // TODO: LDMA_CHx_CFG, LDMA_CHx_LOOP
    }

    /// Get the DMA channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Get channel enabled
    pub fn enabled(&self) -> bool {
        mmio::chen(self.id)
    }

    /// Set channel enabled
    fn set_enabled(&self, is_enabled: bool) {
        if is_enabled {
            mmio::chen_set(self.id());
        } else {
            mmio::chen_clear(self.id());
        }
    }

    /// Get channel busy
    pub fn busy(&self) -> bool {
        mmio::ch_busy(self.id)
    }

    /// Get channel done
    pub fn done(&self) -> bool {
        mmio::ch_done(self.id)
    }

    /// Set channel done
    pub fn set_done(&self, is_done: bool) {
        if is_done {
            mmio::ch_done_set(self.id)
        } else {
            mmio::ch_done_clear(self.id)
        }
    }

    /// Get the channel Peripheral Request selection
    pub fn peripheral_req(&self) -> ChReqSel {
        // # Safety
        //
        // The `LDMA_CHx_REQSEL` can only be written with a safe function from this crate.
        // If the retrieved value is invalid (cannot be converted to `ChReqSel`), then it is reasonable to assume it
        // will have no effect on the peripheral so returning `ChReqSel::None` (the default for `ChReqSel`) makes sense
        mmio::reqsel(self.id).unwrap_or_default()
    }

    /// Set the channel Peripheral Request selection
    pub fn set_peripheral_req(&self, source: ChReqSel) {
        mmio::set_reqsel(self.id, source);
    }

    /// Get channel interrupt enabled
    pub fn ien(&self) -> bool {
        mmio::ien(self.id)
    }

    /// Set channel interrupt enabled
    pub fn set_ien(&self, is_enabled: bool) {
        if is_enabled {
            mmio::ien_set(self.id);
        } else {
            mmio::ien_clear(self.id);
        }
    }

    /// Clear the interrupt flag for this channel
    pub fn set_ifc(&self) {
        mmio::ifc_set(self.id);
    }

    /// Enable channel halt during debugger breakpoint
    pub fn set_dbg_halt(&self) {
        mmio::dbghalt_set(self.id);
    }

    /// Get channel loop count value
    pub fn ch_loop(&self) -> u8 {
        mmio::ch_loop(self.id)
    }

    /// Set channel loop count value
    pub fn set_ch_loop(&self, loop_count: u8) {
        mmio::ch_loop_set(self.id, loop_count);
    }

    /// Set/clear the ignore single requests flag.
    ///
    /// The channel arbiter will ignore single requests (SREQ) and only respond to multiple requests (REQ) when this bit
    /// is set.
    pub fn set_ignore_single_req(&self, is_ignored: bool) {
        mmio::dma().ch(self.id as usize).ctrl().modify(|_, w| {
            if is_ignored {
                w.ignoresreq().set_bit()
            } else {
                w.ignoresreq().clear_bit()
            }
        });
    }

    /// Start the DMA transfer by executing the Transfer LINK Descriptor written to the DMA Channel
    ///
    /// This which will trigger loading the first descriptor in the descriptor list whose address is in the LINK
    /// register
    pub fn link_load(&self) {
        mmio::ch_link_load(self.id)
    }

    /// Write a descriptor to the channel DMA descriptor registers
    pub fn set_descriptor(&self, desc: TransferDescriptor) {
        mmio::ch_write_descriptor(self.id, &desc.into_inner());
    }

    /// Enable the transfer and optionally triggering it
    pub unsafe fn transfer(&mut self, desc: &TransferDescriptor, with_sw_trigger: bool) {
        // cancel any on-going transfers
        self.cancel();

        // Set the non-blocking IRQ handler
        critical_section::with(|cs| {
            set_handler(cs, self.id(), |id, transfer_result| {
                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| irq::irq_ch_set(csd, id, Some(transfer_result)));
            })
        });

        self.start(desc, with_sw_trigger);
    }

    pub fn try_resolve(&mut self) -> Option<Result<(), DmaError>> {
        if let Some(transfer_result) = critical_section::with(|cs| irq::irq_ch_take(cs, self.id)) {
            self.stop();

            Some(transfer_result)
        } else {
            None
        }
    }

    // /// Start a memory-to-memory transfer
    // pub fn into_transfer<'a, W: Sized>(
    //     self,
    //     src: &'a [W],
    //     dst: &'a mut [W],
    // ) -> ChannelTransfer<'a, W> {
    //     let id = self.id;

    //     // Set the IRQ handler for this channel transfer
    //     critical_section::with(|cs| {
    //         set_handler(cs, id, |id, transfer_result| {
    //             // signal to the main thread that transfer is resolved
    //             critical_section::with(|csd| irq::irq_ch_set(csd, id, Some(transfer_result)));
    //         })
    //     });

    //     let mut transfer = ChannelTransfer::new(self, src, dst);
    //     transfer.start();
    //     transfer
    // }

    /// Cancel any on-going transfer
    fn cancel(&mut self) {
        self.stop();
        // Clear any existing content in the IRQ channel of this DMA channel
        critical_section::with(|cs| irq::irq_ch_take(cs, self.id));
    }

    unsafe fn start(&mut self, desc: &TransferDescriptor, with_sw_trigger: bool) {
        self.set_descriptor(*desc);
        self.set_ien(true);
        self.set_enabled(true);
        if with_sw_trigger {
            self.trigger();
        }
    }

    /// Start the DMA transfer by executing the `TransferDescriptor` written to the DMA Channel
    ///
    /// If a descriptor list is linked, it will be executed after the `TransferDescriptor` has finished
    fn trigger(&self) {
        mmio::swreq(self.id);
    }

    fn stop(&mut self) {
        self.set_ien(false);
        self.set_ifc();
        self.set_enabled(false);
        self.set_done(false);
    }
}

/// DMA channel identifier
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChannelId {
    /// Channel 0
    Ch0,
    /// Channel 1
    Ch1,
    /// Channel 2
    Ch2,
    /// Channel 3
    Ch3,
    /// Channel 4
    Ch4,
    /// Channel 5
    Ch5,
    /// Channel 6
    Ch6,
    /// Channel 7
    Ch7,
}

impl ChannelId {
    /// Bitmask for the maximum value of a `ChannelId`
    const MASK_VALUE: u8 = {
        assert!(
            CHANNEL_COUNT.count_ones() == 1,
            "CHANNEL_COUNT must be a power of `2` otherwise the subtraction below won't work"
        );

        CHANNEL_COUNT as u8 - 1
    };

    /// Get a `ChannelId` from a u8
    ///
    /// The caller must make sure the given `val` is valid.
    pub(crate) fn from_u8_unchecked(val: u8) -> Self {
        match val & Self::MASK_VALUE {
            0 => Self::Ch0,
            1 => Self::Ch1,
            2 => Self::Ch2,
            3 => Self::Ch3,
            4 => Self::Ch4,
            5 => Self::Ch5,
            6 => Self::Ch6,
            7 => Self::Ch7,
            _ => unreachable!(),
        }
    }
}

/// Channel Peripheral Request Select
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChReqSel {
    /// No source selected
    #[default]
    None = 0,
    /// Peripheral Reflex System, PRSREQ0
    PrsReq0 = 0x10,
    /// Peripheral Reflex System, PRSREQ1
    PrsReq1 = 0x11,
    /// Analog to Digital Converter 0, ADC0SINGLE REQ/SREQ
    Adc0Single = 0x80,
    /// Analog to Digital Converter 0, ADC0SCAN REQ/SREQ
    Adc0Scan = 0x81,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 0
    /// USART0RXDATAV REQ/SREQ
    Usart0RxDataAvl = 0xC0,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 0
    /// USART0TXBL REQ/SREQ
    Usart0TxBl = 0xC1,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 0
    /// USART0TXEMPTY
    Usart0TxEmpty = 0xC2,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 1
    /// USART1RXDATAV REQ/SREQ
    Usart1RxDataAvl = 0xD0,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 1
    /// USART1TXBL REQ/SREQ
    Usart1TxBl = 0xD1,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 1
    /// USART1TXEMPTY
    Usart1TxEmpty = 0xD2,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 1
    /// USART1RXDATAVRIGHT REQ/SREQ
    Usart1RxDataAvlRight = 0xD3,
    /// Universal Synchronous/Asynchronous Receiver/Transmitter 1
    /// USART1TXBLRIGHT REQ/SREQ
    Usart1TxBlRight = 0xD4,
    /// Low Energy UART 0
    /// LEUART0RXDATAV
    LeUart0RxDataAvl = 0x100,
    /// Low Energy UART 0
    /// LEUART0TXBL
    LeUart0TxBl = 0x101,
    /// Low Energy UART 0
    /// LEUART0TXEMPTY
    LeUart0TxEmpty = 0x102,
    /// I2C 0
    /// I2C0RXDATAV REQ/SREQ
    I2C0RxDataAvl = 0x140,
    /// I2C 0
    /// I2C0TXBL REQ/SREQ
    I2C0TxBl = 0x141,
    /// Timer 0
    /// TIMER0UFOF
    Timer0UfOf = 0x180,
    /// Timer 0
    /// TIMER0CC0
    Timer0Cc0 = 0x181,
    /// Timer 0
    /// TIMER0CC1
    Timer0Cc1 = 0x182,
    /// Timer 0
    /// TIMER0CC2
    Timer0Cc2 = 0x183,
    /// Timer 1
    /// TIMER1UFOF
    Timer1UfOf = 0x190,
    /// Timer 1
    /// TIMER1CC0
    Timer1Cc0 = 0x191,
    /// Timer 1
    /// TIMER1CC1
    Timer1Cc1 = 0x192,
    /// Timer 1
    /// TIMER1CC2
    Timer1Cc2 = 0x193,
    /// Timer 1
    /// TIMER1CC3
    Timer1Cc3 = 0x194,
    /// Memory System Controller
    /// MSCWDATA
    MscWData = 0x300,
    /// Advanced Encryption Standard Accelerator
    /// CRYPTODATA0WR
    CryptoData0Wr = 0x310,
    /// Advanced Encryption Standard Accelerator
    /// CRYPTODATA0XWR
    CryptoData0XWr = 0x311,
    /// Advanced Encryption Standard Accelerator
    /// CRYPTODATA0RD
    CryptoData0Rd = 0x312,
    /// Advanced Encryption Standard Accelerator
    /// CRYPTODATA1WR
    CryptoData1Wr = 0x313,
    /// Advanced Encryption Standard Accelerator
    /// CRYPTODATA1RD
    CryptoData1Rd = 0x314,
}

impl TryFrom<u16> for ChReqSel {
    type Error = DmaError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            0x10 => Ok(Self::PrsReq0),
            0x11 => Ok(Self::PrsReq1),
            0x80 => Ok(Self::Adc0Single),
            0x81 => Ok(Self::Adc0Scan),
            0xC0 => Ok(Self::Usart0RxDataAvl),
            0xC1 => Ok(Self::Usart0TxBl),
            0xC2 => Ok(Self::Usart0TxEmpty),
            0xD0 => Ok(Self::Usart1RxDataAvl),
            0xD1 => Ok(Self::Usart1TxBl),
            0xD2 => Ok(Self::Usart1TxEmpty),
            0xD3 => Ok(Self::Usart1RxDataAvlRight),
            0xD4 => Ok(Self::Usart1TxBlRight),
            0x100 => Ok(Self::LeUart0RxDataAvl),
            0x101 => Ok(Self::LeUart0TxBl),
            0x102 => Ok(Self::LeUart0TxEmpty),
            0x140 => Ok(Self::I2C0RxDataAvl),
            0x141 => Ok(Self::I2C0TxBl),
            0x180 => Ok(Self::Timer0UfOf),
            0x181 => Ok(Self::Timer0Cc0),
            0x182 => Ok(Self::Timer0Cc1),
            0x183 => Ok(Self::Timer0Cc2),
            0x190 => Ok(Self::Timer1UfOf),
            0x191 => Ok(Self::Timer1Cc0),
            0x192 => Ok(Self::Timer1Cc1),
            0x193 => Ok(Self::Timer1Cc2),
            0x194 => Ok(Self::Timer1Cc3),
            0x300 => Ok(Self::MscWData),
            0x310 => Ok(Self::CryptoData0Wr),
            0x311 => Ok(Self::CryptoData0XWr),
            0x312 => Ok(Self::CryptoData0Rd),
            0x313 => Ok(Self::CryptoData1Wr),
            0x314 => Ok(Self::CryptoData1Rd),
            _ => Err(DmaError::InvalidMMIO),
        }
    }
}

/// DMA Error
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DmaError {
    /// The value in a DMA MMIO register is invalid
    InvalidMMIO,
    /// Invalid transfer size (e.g. transfer size is `0`)
    InvalidTransferSize,
    /// DMA transfer failed
    Transfer,
    /// Descriptor list is invalid (e.g empty)
    InvalidDescriptorList,
    /// Descriptor list overflowed
    DescriptorListOverflow,
}

/// DMA interrupt handling
pub mod irq {
    use crate::{
        dma::{mmio, ChannelId, DmaError, CHANNEL_COUNT},
        pac::interrupt,
    };
    use core::cell::RefCell;
    use critical_section::{CriticalSection, Mutex};

    /// Handler function for a DMA interrupt
    type DmaIrqHandler = fn(ChannelId, Result<(), DmaError>);

    /// Handler which does nothing
    const fn default_handler(_: ChannelId, _: Result<(), DmaError>) {}

    /// Communication channels between DMA IRQ and the main thread. One for each `DmaChannel`
    static IRQ_CHANNELS: Mutex<RefCell<[Option<Result<(), DmaError>>; CHANNEL_COUNT]>> =
        Mutex::new(RefCell::new([None; _]));

    /// Interrupt handlers for each DMA Channel
    static HANDLERS: Mutex<RefCell<[DmaIrqHandler; CHANNEL_COUNT]>> =
        Mutex::new(RefCell::new([default_handler; _]));

    pub(crate) fn irq_ch_take(cs: CriticalSection, id: ChannelId) -> Option<Result<(), DmaError>> {
        IRQ_CHANNELS.borrow(cs).borrow_mut()[id as usize].take()
    }

    pub(crate) fn irq_ch_set(
        cs: CriticalSection,
        id: ChannelId,
        new: Option<Result<(), DmaError>>,
    ) {
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
}
