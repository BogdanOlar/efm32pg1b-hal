//! SpiDma async
//!

use crate::{
    dma::{descriptor::UnitSize, transfer::NoParams},
    usart::spi::{
        dma::{RxParam, SpiDma, TxParam},
        SpiError,
    },
};
#[cfg(feature = "debug-spi-dma-defmt-info")]
use defmt::info;
use embassy_futures::join::join;
use embedded_hal_async::spi::SpiBus;

impl SpiDma {
    /// Do an async SPI transaction.
    ///
    /// `write` is written to the slave on MOSI and words received on MISO are stored in `read`.
    pub async fn transfer_async<Word: Copy + 'static>(
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

        let write_units = write_bytes / unit.byte_count();
        let read_units = read_bytes / unit.byte_count();

        // Only do the async DMA transfers if there is something to transfer
        if write_units.max(read_units) > 0 {
            let (tx_desc, rx_desc) =
                self.build_descriptors(unit, write_addr, write_units, read_addr, read_units)?;

            let (tx_res, rx_res) = join(
                self.rx
                    .transfer_async(&rx_desc, false, RxParam { _read: read }),
                self.tx
                    .transfer_async(&tx_desc, true, TxParam { _write: write }),
            )
            .await;

            if tx_res.is_ok() && rx_res.is_ok() {
                Ok(())
            } else if tx_res.is_err() && rx_res.is_err() {
                Err(SpiError::TxRx)
            } else if tx_res.is_err() {
                Err(SpiError::Tx)
            } else {
                Err(SpiError::Rx)
            }
        } else {
            // Zero-sized transfers return OK() immediately without starting DMA
            Ok(())
        }
    }

    /// Do an async SPI transaction.
    ///
    /// The contents of `words` are written to the slave, and the received words are stored into the same `words`
    /// buffer, overwriting it.
    pub async fn transfer_in_place_async<Word: Copy + 'static>(
        &mut self,
        words: &mut [Word],
    ) -> Result<(), SpiError> {
        // FIXME: unit is limited to Byte until we convince the Spi to accept other `UnitSize`s
        let unit = UnitSize::Byte;
        let addr = words.as_ptr().addr();
        let bytes = core::mem::size_of_val(words);

        let write_units = bytes / unit.byte_count();
        let read_units = bytes / unit.byte_count();

        // Only do the async DMA transfers if there is something to transfer
        if write_units.max(read_units) > 0 {
            let (tx_desc, rx_desc) = self.build_descriptors(unit, addr, bytes, addr, bytes)?;

            let (tx_res, rx_res) = join(
                self.rx
                    .transfer_async(&rx_desc, false, RxParam { _read: words }),
                self.tx.transfer_async(&tx_desc, true, NoParams {}),
            )
            .await;

            if tx_res.is_ok() && rx_res.is_ok() {
                Ok(())
            } else if tx_res.is_err() && rx_res.is_err() {
                Err(SpiError::TxRx)
            } else if tx_res.is_err() {
                Err(SpiError::Tx)
            } else {
                Err(SpiError::Rx)
            }
        } else {
            // Zero-sized transfers return OK() immediately without starting DMA
            Ok(())
        }
    }
}

impl SpiBus for SpiDma {
    async fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer_async(words, &[]).await
    }

    async fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.transfer_async(&mut [], words).await
    }

    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.transfer_async(read, write).await
    }

    async fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer_in_place_async(words).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // No idea what an `async flush()` should do, but
        Ok(())
    }
}
