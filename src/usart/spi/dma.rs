//! SPI bus with DMA transfers

use crate::{
    dma::{
        self,
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
    busy: bool,
}

impl<const N: u8> SpiDma<N> {
    pub(crate) fn new(mut tx: DmaChannel, mut rx: DmaChannel) -> Self {
        /// Helper function to get the appropriate peripheral sources based on SPI instance id (`N`):
        /// `(tx_source, rx_source)`
        const fn sources<const N: u8>() -> (ChReqSel, ChReqSel) {
            match N {
                0 => (ChReqSel::Usart0TxBl, ChReqSel::Usart0RxDataAvl),
                1 => (ChReqSel::Usart1TxBl, ChReqSel::Usart1RxDataAvl),
                _ => unreachable!(),
            }
        }

        tx.reset();
        rx.reset();

        let (tx_sel, rx_sel) = sources::<N>();
        tx.set_per_req(tx_sel);
        rx.set_per_req(rx_sel);

        critical_section::with(|cs| {
            // Clear any existing content in the IRQ channel of the DMA channels
            dma::irq::irq_ch_take(cs, tx.id());
            dma::irq::irq_ch_take(cs, rx.id());
            // Set the IRQ handler for TX channel
            dma::irq::set_handler(cs, tx.id(), |id, channel_error| {
                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| dma::irq::irq_ch_set(csd, id, Some(channel_error)));
            })
            // FIXME: Handle RX too?
        });

        Self {
            tx,
            rx,
            busy: false,
        }
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
        if self.busy {
            Err(SpiError::Busy)
        } else {
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

            self.busy = true;

            // start the transfer
            self.tx.set_ien();
            self.tx.set_enable();
            self.tx.start();

            // wait
            while self.tx.busy() || self.rx.busy() {}

            Ok(())
        }
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // FIXME: Handle RX too?

        if self.busy {
            let error = loop {
                if let Some(is_error) =
                    critical_section::with(|cs| dma::irq::irq_ch_take(cs, self.tx.id()))
                {
                    self.busy = false;
                    break is_error;
                }
            };

            // FIXME: don't clear the peripheral source with reset, since that's set only once when the `SpiDma` is
            //        created
            // self.tx.reset();
            // self.rx.reset();

            match error {
                true => Err(SpiError::Tx),
                false => Ok(()),
            }
        } else {
            Ok(())
        }
    }
}

impl<const N: u8> ErrorType for SpiDma<N> {
    type Error = SpiError;
}
