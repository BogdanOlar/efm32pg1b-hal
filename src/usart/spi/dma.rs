//! SPI bus with DMA transfers

use crate::{
    dma::{
        self,
        descriptor::{
            Addr,
            AddrInc::{self, None},
            Descriptor, TransferCount, UnitSize,
        },
        list::{DescList, ImmediateDescBuilder, LoopTransferDescBuilder, TransferDescBuilder},
        ChReqSel, ChannelId, DmaChannel, DmaError,
    },
    usart::{spi::SpiError, usarts::usartx},
};
use embedded_hal::spi::{ErrorType, SpiBus};

/// Maximum number of DMA descriptors in [`SpiDma::descriptors`]
const DESC_COUNT: usize = 10;

/// SPI master which implements `SpiBus` trait
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiDma<const N: u8> {
    tx: DmaChannel,
    rx: DmaChannel,
    busy: bool,
    tx_descriptors: [Descriptor; DESC_COUNT],
    rx_descriptors: [Descriptor; DESC_COUNT],
}

impl<const N: u8> SpiDma<N> {
    pub(crate) fn new(mut tx: DmaChannel, mut rx: DmaChannel) -> Self {
        tx.reset();
        rx.reset();

        let (tx_sel, rx_sel) = Self::sources();
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
            });
            // Set the IRQ handler for TX channel
            dma::irq::set_handler(cs, rx.id(), |id, channel_error| {
                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| dma::irq::irq_ch_set(csd, id, Some(channel_error)));
            });
            // FIXME: Handle RX too?
        });

        Self {
            tx,
            rx,
            busy: false,
            tx_descriptors: [Descriptor::default(); _],
            rx_descriptors: [Descriptor::default(); _],
        }
    }

    /// Helper function to get the appropriate peripheral sources based on SPI instance id (`N`):
    /// Returns `(tx_source, rx_source)`
    const fn sources() -> (ChReqSel, ChReqSel) {
        match N {
            0 => (ChReqSel::Usart0TxBl, ChReqSel::Usart0RxDataAvl),
            1 => (ChReqSel::Usart1TxBl, ChReqSel::Usart1RxDataAvl),
            _ => unreachable!(),
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
        // We may want to make this a generic algo, so let's not hardcode `UnitByte`, or `UnitHalfword`, etc
        let unit = UnitSize::Byte;
        // TX Filler
        static TX_FILLER: u32 = 0xFFFFFFFF;

        let tx_units = write.len() / unit.byte_count();
        assert_eq!(write.len() % unit.byte_count(), 0);
        let rx_units = read.len() / unit.byte_count();
        assert_eq!(read.len() % unit.byte_count(), 0);
        let tx_filler_units = rx_units.saturating_sub(tx_units);
        let total_units = tx_units + tx_filler_units;
        assert_eq!(total_units, rx_units.max(tx_units));

        if self.busy {
            // FIXME: [`embedded_hal::spi::SpiBus`] docs disallow returning a `Busy` error, though it's not clear to me
            //        what the implementation should do. Enqueueing the request is problematic because if one of the
            //        queued transfer fails, then how is the user supposed to know which one failed?
            Err(SpiError::Busy)
        } else if total_units == 0 {
            // FIXME: make sure the tx DMA channel gets its "done" token
            Ok(())
        } else {
            let usart_p = usartx::<N>();
            let mut tx_list = DescList::new(&mut self.tx_descriptors);
            let mut rx_list = DescList::new(&mut self.rx_descriptors);

            fn extended(
                dma_ch_id: ChannelId,
                src_addr: usize,
                src_addr_inc: AddrInc,
                dst_addr: usize,
                dst_addr_inc: AddrInc,
                unit: UnitSize,
                unit_count: usize,
                total_unit_count: usize,
                mut desc_list: DescList,
            ) -> Result<DescList, DmaError> {
                const NON_LOOP_TRANSFER_COUNT: usize = 2;
                const MAX_TRANSFER_COUNT: usize =
                    u8::MAX as usize * Descriptor::MAX_TRANSFER_UNITS + NON_LOOP_TRANSFER_COUNT;

                let remainder = unit_count % Descriptor::MAX_TRANSFER_UNITS;
                let transfer_count = if remainder == 0 {
                    unit_count / Descriptor::MAX_TRANSFER_UNITS
                } else {
                    unit_count / Descriptor::MAX_TRANSFER_UNITS + 1
                };

                if transfer_count > MAX_TRANSFER_COUNT {
                    return Err(DmaError::InvalidTransferSize(dma_ch_id));
                }

                let loop_count = transfer_count.saturating_sub(NON_LOOP_TRANSFER_COUNT);

                // Immediate Transfer needs to be written before the first Transfer because the first Transfer will
                // set the absolute address of the buffer, so that the rest of the Transfers can use relative addressing
                if loop_count > 0 {
                    desc_list = desc_list.push(ImmediateDescBuilder::new(
                        (loop_count - 1) as u32,
                        crate::dma::mmio::dma()
                            .ch(dma_ch_id as usize)
                            .loop_()
                            .as_ptr()
                            .addr(),
                    ))?;
                }

                desc_list = desc_list.push(
                    TransferDescBuilder::new(
                        Addr::Absolute(src_addr),
                        Addr::Absolute(dst_addr),
                        if remainder > 0 {
                            remainder.try_into().unwrap()
                        } else {
                            TransferCount::MAX
                        },
                        unit,
                    )
                    .with_src_inc(src_addr_inc)
                    .with_dst_inc(dst_addr_inc)
                    .with_done_ifs(transfer_count == 1),
                )?;

                if transfer_count > 1 {
                    if loop_count > 0 {
                        desc_list = desc_list.push(
                            LoopTransferDescBuilder::new(
                                Addr::Relative(0),
                                Addr::Relative(0),
                                TransferCount::MAX,
                                Addr::Relative(0),
                                unit,
                            )
                            .with_src_inc(src_addr_inc)
                            .with_dst_inc(dst_addr_inc),
                        )?;
                    }

                    desc_list = desc_list.push(
                        TransferDescBuilder::new(
                            Addr::Relative(0),
                            Addr::Relative(0),
                            TransferCount::MAX,
                            unit,
                        )
                        .with_src_inc(src_addr_inc)
                        .with_dst_inc(dst_addr_inc)
                        .with_done_ifs(total_unit_count == unit_count),
                    )?;
                }

                Ok(desc_list)
            }

            // TX
            if tx_units > 0 {
                tx_list = extended(
                    self.tx.id(),
                    write.as_ptr().addr(),
                    AddrInc::One,
                    usart_p.txdata().as_ptr().addr(),
                    AddrInc::None,
                    unit,
                    tx_units,
                    total_units,
                    tx_list,
                )?;
            }

            // TX filler
            if tx_filler_units > 0 {
                tx_list = extended(
                    self.tx.id(),
                    (&TX_FILLER) as *const u32 as usize,
                    AddrInc::None,
                    usart_p.txdata().as_ptr().addr(),
                    AddrInc::None,
                    unit,
                    tx_filler_units,
                    tx_filler_units + tx_units,
                    tx_list,
                )?;
            }

            // RX
            if rx_units > 0 {
                rx_list = extended(
                    self.rx.id(),
                    usart_p.rxdata().as_ptr().addr(),
                    AddrInc::None,
                    read.as_ptr().addr(),
                    AddrInc::One,
                    unit,
                    rx_units,
                    total_units,
                    rx_list,
                )?;
            }

            // start the transfer
            self.busy = true;

            self.rx.set_descriptor(&rx_list.finalize());
            self.rx.set_ien();
            self.rx.set_enable();
            self.rx.link_load();

            self.tx.set_descriptor(&tx_list.finalize());
            self.tx.set_ien();
            self.tx.set_enable();
            self.tx.link_load();

            // // FIXME: why does this cause `tests/spi.rs` `transfer_u8_dma_short()` test to fail?! Fishy, fishy!
            // while self.tx.busy() || self.rx.busy() {}

            Ok(())
        }
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // FIXME: Handle RX too?

        if self.busy {
            let tx_error = loop {
                if let Some(is_error) =
                    critical_section::with(|cs| dma::irq::irq_ch_take(cs, self.tx.id()))
                {
                    break is_error;
                }
            };

            let rx_error = loop {
                if let Some(is_error) =
                    critical_section::with(|cs| dma::irq::irq_ch_take(cs, self.rx.id()))
                {
                    break is_error;
                }
            };

            self.busy = false;
            // FIXME: don't clear the peripheral source with reset, since that's set only once when the `SpiDma` is
            //        created
            // self.tx.reset();
            // self.rx.reset();

            match tx_error || rx_error {
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
