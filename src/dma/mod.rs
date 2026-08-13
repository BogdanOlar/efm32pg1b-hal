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

#[cfg(feature = "efemb")]
pub mod efemb;

use crate::{
    dma::{
        descriptor::{Addr, Descriptor, TransferDescriptor, UnitSize},
        irq::set_handler,
    },
    pac::{Interrupt, Ldma, NVIC},
};
use core::cmp::min;
use cortex_m::asm;

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
    /// Number of DMA channels
    const COUNT: usize = 1 << 3;

    /// Reset channel to a known state
    pub fn reset(&mut self) {
        mmio::chen_clear(self.id);
        mmio::ien_clear(self.id);
        mmio::ifc_set(self.id);
        mmio::chdone_clear(self.id);

        mmio::ctrl_syncprsseten_clear(self.id);
        mmio::ctrl_syncprsclren_clear(self.id);
        mmio::sync_clear(self.id);
        mmio::dbghalt_clear(self.id);
        mmio::reqdis_clear(self.id);
        mmio::reqclear_set(self.id);
        mmio::reqsel_set(self.id, ChReqSel::None);

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

    /// Enable channel
    pub fn set_enable(&self) {
        mmio::chen_set(self.id());
    }
    /// Disable channel
    pub fn clear_enable(&self) {
        mmio::chen_clear(self.id());
    }

    /// Get channel busy (if enabled)
    pub fn busy(&self) -> bool {
        self.enabled() && mmio::ch_busy(self.id)
    }

    /// Start a memory-to-memory transfer
    pub fn into_transfer<'a, W: Sized>(
        self,
        src: &'a [W],
        dst: &'a mut [W],
    ) -> ChannelTransfer<'a, W> {
        let id = self.id;

        // Set the IRQ handler for this channel transfer
        critical_section::with(|cs| {
            set_handler(cs, id, |id, channel_error| {
                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| irq::irq_ch_set(csd, id, Some(channel_error)));
            })
        });

        let mut transfer = ChannelTransfer::new(self, src, dst);
        transfer.start();
        transfer
    }

    /// Set the channel Peripheral Request selection
    pub fn set_per_req(&self, source: ChReqSel) {
        mmio::reqsel_set(self.id, source);
    }

    /// Write a descriptor to the channel DMA descriptor registers
    pub fn set_descriptor(&self, descr: TransferDescriptor) {
        mmio::ch_write_descriptor(self.id, &descr.into_inner());
    }

    /// Enable channel interrupt
    pub fn set_ien(&self) {
        mmio::ien_set(self.id);
    }

    /// Disable channel interrupt
    pub fn clear_ien(&self) {
        mmio::ien_clear(self.id);
    }

    /// Enable channel halt during debugger breakpoint
    pub fn set_dbg_halt(&self) {
        mmio::dbghalt_set(self.id);
    }

    pub fn start(&self) {
        mmio::swreq(self.id);
    }

    pub fn link_load(&self) {
        mmio::ch_link_load(self.id)
    }

    /// Set channel loop count value
    pub fn set_ch_loop(&self, loop_count: u8) {
        mmio::ch_loop_set(self.id, loop_count);
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
            DmaChannel::COUNT.count_ones() == 1,
            "DmaChannel::COUNT must be a power of `2` otherwise the subtraction below won't work"
        );

        DmaChannel::COUNT as u8 - 1
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChReqSel {
    /// No source selected
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
    fn new(ch: DmaChannel, src: &'a [W], dst: &'a mut [W]) -> Self {
        let byte_count = min(core::mem::size_of_val(src), core::mem::size_of_val(dst));

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

    /// Start the DMA transfer
    fn start(&mut self) {
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
            min(
                Descriptor::MAX_TRANSFER_UNITS,
                remaining_units - last_chunk_min_units,
            )
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
                min(
                    Descriptor::MAX_TRANSFER_UNITS,
                    remaining_units - last_chunk_min_units,
                )
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

/// DMA Error
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DmaError {
    /// Invalid transfer size (e.g. transfer size is `0`)
    InvalidTransferSize(ChannelId),
    /// DMA transfer failed
    Transfer(DmaChannel),
    /// Descriptor list overflowed
    DescriptorListOverflow,
}

/// DMA interrupt handling
pub mod irq {
    use crate::{
        dma::{mmio, ChannelId, DmaChannel},
        pac::interrupt,
    };
    use core::cell::RefCell;
    use critical_section::{CriticalSection, Mutex};

    /// Handler function for a DMA interrupt
    type DmaIrqHandler = fn(ChannelId, bool);

    /// Handler which does nothing
    const fn default_handler(_: ChannelId, _: bool) {}

    /// Communication channels between DMA IRQ and the main thread. One for each `DmaChannel`
    static IRQ_CHANNELS: Mutex<RefCell<[Option<bool>; DmaChannel::COUNT]>> =
        Mutex::new(RefCell::new([None; _]));

    /// Interrupt handlers for each DMA Channel
    static HANDLERS: Mutex<RefCell<[DmaIrqHandler; DmaChannel::COUNT]>> =
        Mutex::new(RefCell::new([default_handler; _]));

    pub(crate) fn irq_ch_take(cs: CriticalSection, id: ChannelId) -> Option<bool> {
        IRQ_CHANNELS.borrow(cs).borrow_mut()[id as usize].take()
    }

    pub(crate) fn irq_ch_set(cs: CriticalSection, id: ChannelId, new: Option<bool>) {
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
        if let Some(id) = mmio::if_error() {
            mmio::if_error_clear();
            let handle = critical_section::with(|cs| HANDLERS.borrow(cs).borrow()[id as usize]);
            handle(id, true);
        }

        // process channel done flags
        for id in mmio::if_raised() {
            mmio::ifc_set(id);
            let handle = critical_section::with(|cs| HANDLERS.borrow(cs).borrow()[id as usize]);
            handle(id, false);
        }
    }
}

/// Register-level DMA functions
pub(crate) mod mmio {
    use crate::dma::descriptor::Descriptor;
    use crate::dma::{ChReqSel, ChannelId, DmaChannel};
    use crate::pac::Ldma;
    use crate::SingleCycleRMW;

    /// Disable "Synchronization PRS Set Enable"
    pub(crate) fn ctrl_syncprsseten_clear(id: ChannelId) {
        dma().ctrl().sc_clear(1 << id as u8);
    }

    /// Disable "Synchronization PRS Clear Enable"
    pub(crate) fn ctrl_syncprsclren_clear(id: ChannelId) {
        dma().ctrl().sc_clear((1 << id as u8) << DmaChannel::COUNT);
    }

    pub(crate) fn sync_clear(id: ChannelId) {
        dma().sync().sc_clear(1 << id as u8);
    }

    /// Get channel enabled
    pub(crate) fn chen(id: ChannelId) -> bool {
        dma().chen().read().chen().bits() & (1 << id as u8) != 0
    }

    /// Enable channel
    pub(crate) fn chen_set(id: ChannelId) {
        dma().chen().sc_set(1 << id as u8);
    }

    /// Disable channel
    pub(crate) fn chen_clear(id: ChannelId) {
        dma().chen().sc_clear(1 << id as u8);
    }

    pub(crate) fn ch_done(id: ChannelId) -> bool {
        dma().chdone().read().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn ch_done_set(id: ChannelId) {
        dma().chdone().sc_set(1 << id as u8);
    }

    pub(crate) fn chdone_clear(id: ChannelId) {
        dma().chdone().sc_clear(1 << id as u8);
    }

    pub(crate) fn dbghalt_clear(id: ChannelId) {
        dma().dbghalt().sc_clear(1 << id as u8);
    }

    pub(crate) fn dbghalt_set(id: ChannelId) {
        dma().dbghalt().sc_set(1 << id as u8);
    }

    pub(crate) fn reqdis_clear(id: ChannelId) {
        dma().reqdis().sc_clear(1 << id as u8);
    }

    pub(crate) fn reqclear_set(id: ChannelId) {
        dma().reqclear().sc_set(1 << id as u8);
    }

    pub(crate) fn ch_error(id: ChannelId) -> bool {
        dma().status().read().cherror().bits() == id as u8
    }

    pub(crate) fn ch_busy(id: ChannelId) -> bool {
        dma().chbusy().read().busy().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn ien_set(id: ChannelId) {
        dma().ien().sc_set(1 << id as u8);
    }

    pub(crate) fn ien_clear(id: ChannelId) {
        dma().ien().sc_clear(1 << id as u8);
    }

    pub(crate) fn ifc_set(id: ChannelId) {
        dma().ifc().sc_set(1 << id as u8);
    }

    pub(crate) fn if_error() -> Option<ChannelId> {
        if dma().if_().read().error().bit_is_set() {
            Some(ChannelId::from_u8_unchecked(
                dma().status().read().cherror().bits(),
            ))
        } else {
            None
        }
    }

    pub(crate) fn if_error_clear() {
        dma().ifc().write(|w| w.error().set_bit());
    }

    pub(crate) fn swreq(id: ChannelId) {
        dma()
            .swreq()
            .write(|w| unsafe { w.swreq().bits(1 << id as u8) });
    }

    pub(crate) fn ch_loop_set(id: ChannelId, loop_count: u8) {
        dma()
            .ch(id as usize)
            .loop_()
            .write(|w| unsafe { w.loopcnt().bits(loop_count) });
    }

    /// Set Channel Peripheral Request Select
    pub(crate) fn reqsel_set(id: ChannelId, source: ChReqSel) {
        let sig = ((source as u16) & 0b1111) as u8;
        let source = (((source as u16) >> 4) & 0b111111) as u8;

        dma()
            .ch(id as usize)
            .reqsel()
            .write(|w| unsafe { w.sigsel().bits(sig).sourcesel().bits(source) });
    }

    pub(crate) fn ch_link_load(id: ChannelId) {
        dma()
            .linkload()
            .write(|w| unsafe { w.linkload().bits(1 << id as u8) });
    }

    pub(crate) fn ch_req_mode_set(id: ChannelId, all: bool) {
        dma()
            .ch(id as usize)
            .ctrl()
            .modify(|_, w| w.reqmode().bit(all));
    }

    /// WARNING: number of words actually transfered will be `cnt + 1`
    pub(crate) fn ch_xfer_cnt_set(id: ChannelId, cnt: u16) {
        dma()
            .ch(id as usize)
            .ctrl()
            .write(|w| unsafe { w.xfercnt().bits(cnt) });
    }

    pub(crate) fn ch_src_set(id: ChannelId, addr: u32) {
        dma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.srcaddr().bits(addr) });
    }

    pub(crate) fn ch_dst_set(id: ChannelId, addr: u32) {
        dma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.dstaddr().bits(addr) });
    }

    pub(crate) fn ch_write_descriptor(id: ChannelId, descr: &Descriptor) {
        dma()
            .ch(id as usize)
            .ctrl()
            .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_CTRL]) });
        dma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_SRC]) });
        dma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_DST]) });
        dma()
            .ch(id as usize)
            .link()
            .write(|w| unsafe { w.bits(descr.raw[Descriptor::INDEX_LINK]) });
    }

    /// Iterator over all raised channel DMA done flags
    pub(crate) fn if_raised() -> impl Iterator<Item = ChannelId> {
        let cached_flags = dma().if_().read().done().bits();

        (0..DmaChannel::COUNT as u8)
            .filter(move |i| ((1 << *i) & cached_flags) != 0)
            .map(ChannelId::from_u8_unchecked)
    }

    /// Get the DMA (pac) peripheral
    pub(crate) fn dma() -> Ldma {
        unsafe { crate::pac::Ldma::steal() }
    }
}
