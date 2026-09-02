//! SPI bus with DMA transfers

use crate::{
    dma::{
        self,
        descriptor::{Descriptor, TransferDescriptor, UnitSize},
        list::{DescList, FinMode, TargetAddr},
        ChReqSel, DmaChannel, DmaError,
    },
    usart::{
        mmio,
        spi::{Spi, SpiError, TX_FILLER_BYTE},
        UsartId,
    },
};
#[cfg(feature = "debug-spi-dma-defmt-info")]
use defmt::info;
use embedded_hal::spi::{ErrorType, SpiBus};

/// Maximum number of DMA descriptors in [`SpiDma::descriptors`]
const DESC_COUNT: usize = 6;

/// SPI master which implements `SpiBus` trait
///
/// Owns the [`Spi`](super::Spi) it was created from (extracting its [`UsartId`](super::super::UsartId)
/// for register access), plus two DMA channels and their descriptor pools. Created from an
/// [`Spi`](super::Spi) via [`Spi::into_spi_dma`](super::Spi::into_spi_dma).
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiDma {
    pub(crate) spi: Spi,
    pub(crate) tx: DmaChannel,
    pub(crate) rx: DmaChannel,
    pub(crate) busy: bool,
    pub(crate) tx_descriptors: [Descriptor; DESC_COUNT],
    pub(crate) rx_descriptors: [Descriptor; DESC_COUNT],
}

impl SpiDma {
    pub(crate) fn new(spi: Spi, mut tx: DmaChannel, mut rx: DmaChannel) -> Self {
        tx.reset();
        rx.reset();

        let (tx_sel, rx_sel) = Self::dma_sources(spi.id());
        tx.set_peripheral_req(tx_sel);
        tx.set_ignore_single_req(true);
        rx.set_peripheral_req(rx_sel);
        rx.set_ignore_single_req(true);

        Self {
            spi,
            tx,
            rx,
            busy: false,
            tx_descriptors: [Descriptor::default(); _],
            rx_descriptors: [Descriptor::default(); _],
        }
    }

    /// Start an SPI transaction.
    ///
    /// `write` is written to the slave on MOSI and words received on MISO are stored in `read`.
    pub fn transfer_nb<Word: Copy + 'static>(
        &mut self,
        read: &mut [Word],
        write: &[Word],
    ) -> Result<(), SpiError> {
        // FIXME: unit is limited to Byte until we convince the Spi to accept other `UnitSize`s
        let unit = UnitSize::Byte;
        let write_addr = write.as_ptr().addr();
        let write_bytes = core::mem::size_of_val(write);
        let read_addr = read.as_ptr().addr();
        let read_bytes = core::mem::size_of_val(read);

        self.transfer_inner(unit, write_addr, write_bytes, read_addr, read_bytes)
    }

    /// Start an SPI transaction.
    ///
    /// The contents of `words` are written to the slave, and the received words are stored into the same `words`
    /// buffer, overwriting it.
    pub fn transfer_in_place_nb<Word: Copy + 'static>(
        &mut self,
        words: &mut [Word],
    ) -> Result<(), SpiError> {
        // FIXME: unit is limited to Byte until we convince the Spi to accept other `UnitSize`s
        let unit = UnitSize::Byte;
        let addr = words.as_ptr().addr();
        let bytes = core::mem::size_of_val(words);

        self.transfer_inner(unit, addr, bytes, addr, bytes)
    }

    /// Wait until all operations have completed and the bus is idle.
    pub fn flush_blocking(&mut self) -> Result<(), SpiError> {
        if self.busy {
            let tx_result = loop {
                if let Some(result) = self.tx.try_resolve() {
                    break result;
                }
            };

            let rx_result = loop {
                if let Some(result) = self.rx.try_resolve() {
                    break result;
                }
            };

            if tx_result.is_ok() && rx_result.is_ok() {
                Ok(())
            } else {
                Err(SpiError::Dma(DmaError::Transfer))
            }
        } else {
            Ok(())
        }
    }

    /// Helpher method to start the transfer without involving the read/write slices, just their address and size
    fn transfer_inner(
        &mut self,
        unit: UnitSize,
        write_addr: usize,
        write_bytes: usize,
        read_addr: usize,
        read_bytes: usize,
    ) -> Result<(), SpiError> {
        let write_units = write_bytes / unit.byte_count();
        let read_units = read_bytes / unit.byte_count();

        // Only start DMA transfers if there is something to transfer
        if write_units.max(read_units) > 0 {
            self.busy = true;

            let (tx_desc, rx_desc) =
                self.build_descriptors(unit, write_addr, write_units, read_addr, read_units)?;

            unsafe { self.rx.transfer(&rx_desc, false) };
            unsafe { self.tx.transfer(&tx_desc, true) };
        }

        Ok(())
    }

    /// Helper method to create the descriptor list for an SPI transaction
    ///
    /// Returns the TX and RX descriptors which can be used to start DMA transfers
    pub(crate) fn build_descriptors(
        &mut self,
        unit: UnitSize,
        tx_addr: usize,
        tx_units: usize,
        rx_addr: usize,
        rx_units: usize,
    ) -> Result<(TransferDescriptor, TransferDescriptor), SpiError> {
        let total_units = tx_units.max(rx_units);
        assert_ne!(total_units, 0);

        #[cfg(feature = "debug-spi-dma-defmt-info")]
        info!("UNIT {}", unit);

        let usart_p = mmio::usartx(self.spi.id());
        let mut tx_list = DescList::new(&mut self.tx_descriptors);
        let mut rx_list = DescList::new(&mut self.rx_descriptors);

        let tx_filler_units = total_units - tx_units;
        let rx_filler_units = total_units - rx_units;

        static TX_FILLER: u32 = u32::from_le_bytes([
            TX_FILLER_BYTE,
            TX_FILLER_BYTE,
            TX_FILLER_BYTE,
            TX_FILLER_BYTE,
        ]);
        static mut RX_SINK: u32 = 0;

        let per_tx_addr = usart_p.txdata().as_ptr().addr();
        let per_rx_addr = usart_p.rxdata().as_ptr().addr();

        let tx_desc = if tx_units > 0 {
            #[cfg(feature = "debug-spi-dma-defmt-info")]
            info!("TX: tx_units {}", tx_units);

            let desc = dma::list::reduced(
                self.tx.id(),
                TargetAddr::IncrementOne(tx_addr),
                TargetAddr::Fixed(per_tx_addr),
                unit,
                tx_units,
                &mut tx_list,
            )?;

            if tx_filler_units > 0 {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("TX tx_filler_units {}", tx_filler_units);

                dma::list::extended(
                    self.tx.id(),
                    TargetAddr::Fixed((&TX_FILLER) as *const u32 as usize),
                    TargetAddr::Fixed(per_tx_addr),
                    unit,
                    tx_filler_units,
                    &mut tx_list,
                )?;
            }

            tx_list.into_transfer_descriptor(desc, FinMode::DoneIFS)
        } else {
            #[cfg(feature = "debug-spi-dma-defmt-info")]
            info!("TX tx_filler_units {}", tx_filler_units);

            let desc = dma::list::reduced(
                self.tx.id(),
                TargetAddr::Fixed((&TX_FILLER) as *const u32 as usize),
                TargetAddr::Fixed(per_tx_addr),
                unit,
                tx_filler_units,
                &mut tx_list,
            )?;

            tx_list.into_transfer_descriptor(desc, FinMode::DoneIFS)
        };

        let rx_desc = if rx_units > 0 {
            #[cfg(feature = "debug-spi-dma-defmt-info")]
            info!("RX: rx_units {}", rx_units);

            let desc = dma::list::reduced(
                self.rx.id(),
                TargetAddr::Fixed(per_rx_addr),
                TargetAddr::IncrementOne(rx_addr),
                unit,
                rx_units,
                &mut rx_list,
            )?;

            if rx_filler_units > 0 {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("RX rx_filler_units {}", tx_filler_units);

                dma::list::extended(
                    self.rx.id(),
                    TargetAddr::Fixed(per_rx_addr),
                    TargetAddr::Fixed((&raw const RX_SINK) as usize),
                    unit,
                    rx_filler_units,
                    &mut rx_list,
                )?;
            }

            rx_list.into_transfer_descriptor(desc, FinMode::DoneIFS)
        } else {
            #[cfg(feature = "debug-spi-dma-defmt-info")]
            info!("RX tx_filler_units {}", tx_filler_units);

            let desc = dma::list::reduced(
                self.rx.id(),
                TargetAddr::Fixed(per_rx_addr),
                TargetAddr::Fixed((&raw const RX_SINK) as usize),
                unit,
                rx_filler_units,
                &mut rx_list,
            )?;

            rx_list.into_transfer_descriptor(desc, FinMode::DoneIFS)
        };

        Ok((tx_desc, rx_desc))
    }

    /// Helper function to get the appropriate peripheral DMA channel trigger sources based on
    /// the USART peripheral [`UsartId`]. Returns `(tx_source, rx_source)`.
    pub(crate) const fn dma_sources(id: UsartId) -> (ChReqSel, ChReqSel) {
        match id {
            UsartId::Usart0 => (ChReqSel::Usart0TxBl, ChReqSel::Usart0RxDataAvl),
            UsartId::Usart1 => (ChReqSel::Usart1TxBl, ChReqSel::Usart1RxDataAvl),
        }
    }
}

impl SpiBus for SpiDma {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer(words, &[])
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.transfer(&mut [], words)
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.transfer_nb(read, write)
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer_in_place_nb(words)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_blocking()
    }
}

impl ErrorType for SpiDma {
    type Error = SpiError;
}
