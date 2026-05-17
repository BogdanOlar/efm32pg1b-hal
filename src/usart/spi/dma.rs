//! SPI bus with DMA transfers

use crate::{
    dma::{
        descriptor::{Addr, AddrInc, TransferDescBuilder, UnitByte},
        ChReqSel, DmaChannel,
    },
    usart::{spi::SpiError, usarts::usartx},
};
use embedded_hal::spi::{ErrorType, SpiBus};

/// SPI master which implements `SpiBus` trait
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiDma<const N: u8> {
    tx: DmaChannel,
    rx: DmaChannel,
}

impl<const N: u8> SpiDma<N> {
    pub(crate) fn new(tx: DmaChannel, rx: DmaChannel) -> Self {
        let (tx_sel, rx_sel) = match N & 1 {
            0 => (ChReqSel::Usart0TxBl, ChReqSel::Usart0RxDataV),
            1 => (ChReqSel::Usart1TxBl, ChReqSel::Usart1RxDataV),
            _ => unreachable!(),
        };

        tx.set_per_req(tx_sel);
        rx.set_per_req(rx_sel);

        Self { tx, rx }
    }
}

impl<const N: u8> SpiBus for SpiDma<N> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        let unit_cout = write.len().min(read.len());
        let usart_p = usartx::<N>();

        let rx_desc = unsafe {
            TransferDescBuilder::<UnitByte>::new(
                Addr::Absolute(usart_p.rxdata().as_ptr().addr()),
                Addr::Absolute(read.as_ptr().addr()),
                unit_cout.try_into().unwrap(),
            )
        }
        .with_done_ifs()
        .with_src_inc(AddrInc::None)
        .with_dst_inc(AddrInc::One)
        .build();
        self.rx.set_descriptor(&rx_desc);

        // start the transfer
        self.rx.set_ien();
        self.rx.set_enable();
        // self.rx.start();

        let tx_desc = unsafe {
            TransferDescBuilder::<UnitByte>::new(
                Addr::Absolute(write.as_ptr().addr()),
                Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                unit_cout.try_into().unwrap(),
            )
        }
        .with_done_ifs()
        .with_src_inc(AddrInc::One)
        .with_dst_inc(AddrInc::None)
        .build();
        self.tx.set_descriptor(&tx_desc);

        // start the transfer
        self.tx.set_ien();
        self.tx.set_enable();
        self.tx.start();

        // wait
        while self.tx.busy() || self.rx.busy() {}

        Ok(())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

impl<const N: u8> ErrorType for SpiDma<N> {
    type Error = SpiError;
}
