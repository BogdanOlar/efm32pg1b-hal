//! SPI bus with DMA transfers

use crate::{
    dma::{
        self,
        descriptor::{
            Addr,
            AddrInc::{self},
            Descriptor, ImmediateDescriptor, LoopTransferDescriptor, TransferCount,
            TransferDescriptor, UnitSize,
        },
        list::DescList,
        ChReqSel, ChannelId, DmaChannel, DmaError,
    },
    usart::{mmio, spi::SpiError},
};
#[cfg(feature = "debug-spi-dma-defmt-info")]
use defmt::info;
use embedded_hal::spi::{ErrorType, SpiBus};

/// Maximum number of DMA descriptors in [`SpiDma::descriptors`]
const DESC_COUNT: usize = 6;

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
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("IRQ Tx (error: {})", channel_error);

                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| dma::irq::irq_ch_set(csd, id, Some(channel_error)));
            });
            // Set the IRQ handler for RX channel
            dma::irq::set_handler(cs, rx.id(), |id, channel_error| {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("IRQ Rx (error: {})", channel_error);

                // signal to the main thread that transfer is resolved
                critical_section::with(|csd| dma::irq::irq_ch_set(csd, id, Some(channel_error)));
            });
        });

        Self {
            tx,
            rx,
            busy: false,
            tx_descriptors: [Descriptor::default(); _],
            rx_descriptors: [Descriptor::default(); _],
        }
    }

    /// Helper function to get the appropriate peripheral DMA channel trigger sources based on SPI instance id (`N`):
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
        self.transfer(words, &[])
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.transfer(&mut [], words)
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
            let usart_p = mmio::usartx::<N>();
            let mut tx_list = DescList::new(&mut self.tx_descriptors);
            let rx_list = DescList::new(&mut self.rx_descriptors);

            // TX
            if tx_units > 0 {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("TX: tx_units {}", tx_units);

                let (mut desc, list) = reduced(
                    self.tx.id(),
                    write.as_ptr().addr(),
                    AddrInc::One,
                    usart_p.txdata().as_ptr().addr(),
                    AddrInc::None,
                    unit,
                    tx_units,
                    total_units == tx_units,
                    tx_list,
                )?;
                tx_list = list;

                if tx_filler_units > 0 {
                    #[cfg(feature = "debug-spi-dma-defmt-info")]
                    info!("TX tx_filler_units {}", tx_filler_units);

                    let list = extended(
                        self.tx.id(),
                        (&TX_FILLER) as *const u32 as usize,
                        AddrInc::None,
                        usart_p.txdata().as_ptr().addr(),
                        AddrInc::None,
                        unit,
                        tx_filler_units,
                        true,
                        tx_list,
                    )?;
                    tx_list = list;
                    desc = desc.with_link(Addr::Absolute(tx_list.storage_addr()), true);
                }

                self.tx.set_descriptor(desc);
            } else if tx_filler_units > 0 {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("TX tx_filler_units {}", tx_filler_units);

                let (desc, _) = reduced(
                    self.tx.id(),
                    (&TX_FILLER) as *const u32 as usize,
                    AddrInc::None,
                    usart_p.txdata().as_ptr().addr(),
                    AddrInc::None,
                    unit,
                    tx_filler_units,
                    true,
                    tx_list,
                )?;
                self.tx.set_descriptor(desc);
            }

            // RX
            if rx_units > 0 {
                #[cfg(feature = "debug-spi-dma-defmt-info")]
                info!("RX: rx_units {}", rx_units);

                let (desc, _) = reduced(
                    self.rx.id(),
                    usart_p.rxdata().as_ptr().addr(),
                    AddrInc::None,
                    read.as_ptr().addr(),
                    AddrInc::One,
                    unit,
                    rx_units,
                    true,
                    rx_list,
                )?;
                self.rx.set_descriptor(desc);
            }

            // start the transfer
            self.busy = true;

            // self.rx.set_dbg_halt();
            self.rx.set_ien();
            self.rx.set_enable();
            self.tx.start();

            // self.tx.set_dbg_halt();
            self.tx.set_ien();
            self.tx.set_enable();
            self.tx.start();

            Ok(())
        }
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
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
            self.tx.clear_enable();
            self.tx.clear_ien();
            self.rx.clear_enable();
            self.rx.clear_ien();

            // FIXME: When the RX Dma ends followed by the TX Dma, then there are still SPI TX fifo bytes being
            //        transacted, which causes the RX Spi buffer to not be empty when the next DMA transaction starts
            //        causing spurious bytes to be received.
            //        This should be done on the SPI driver level, not PAC level
            {
                let usart = mmio::usartx::<N>();
                // wait for SPI TX to end
                while usart.status().read().txidle().bit_is_clear() {}
                // flush RX SPI buffer
                while usart.status().read().rxdatav().bit_is_set() {
                    let _ = usart.rxdata().read().rxdata().bits();
                }
            }

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

fn reduced(
    dma_ch_id: ChannelId,
    src_addr: usize,
    src_addr_inc: AddrInc,
    dst_addr: usize,
    dst_addr_inc: AddrInc,
    unit: UnitSize,
    unit_count: usize,
    is_done: bool,
    mut desc_list: DescList,
) -> Result<(TransferDescriptor, DescList), DmaError> {
    const NON_LOOP_TRANSFER_COUNT: usize = 2;
    const MAX_LOOP_COUNT: usize = u8::MAX as usize;
    const MAX_TRANSFER_COUNT: usize =
        MAX_LOOP_COUNT * Descriptor::MAX_TRANSFER_UNITS + NON_LOOP_TRANSFER_COUNT;

    let remainder = unit_count % Descriptor::MAX_TRANSFER_UNITS;
    let transfer_count = if remainder == 0 {
        unit_count / Descriptor::MAX_TRANSFER_UNITS
    } else {
        unit_count / Descriptor::MAX_TRANSFER_UNITS + 1
    };

    if transfer_count > MAX_TRANSFER_COUNT {
        return Err(DmaError::InvalidTransferSize(dma_ch_id));
    }

    let iteration_count = transfer_count.saturating_sub(NON_LOOP_TRANSFER_COUNT);

    // Write DMA channel loop count
    if iteration_count > 0 {
        let loop_count = iteration_count - 1;

        #[cfg(feature = "debug-spi-dma-defmt-info")]
        info!("\t loop_count: {}", loop_count);

        crate::dma::mmio::dma()
            .ch(dma_ch_id as usize)
            .loop_()
            .write(|w| unsafe { w.loopcnt().bits(loop_count as u8) });
    }

    let raw_desc = TransferDescriptor::new(
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
    .with_done_ifs((transfer_count == 1) && is_done)
    .with_link(Addr::Absolute(desc_list.storage_addr()), transfer_count > 1);

    if transfer_count > 1 {
        #[cfg(feature = "debug-spi-dma-defmt-info")]
        info!("\t transfer_count: {}", &transfer_count);

        if iteration_count > 0 {
            desc_list = desc_list.push_linked(
                LoopTransferDescriptor::new(
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

        desc_list = desc_list.push_linked(
            TransferDescriptor::new(
                Addr::Relative(0),
                Addr::Relative(0),
                TransferCount::MAX,
                unit,
            )
            .with_src_inc(src_addr_inc)
            .with_dst_inc(dst_addr_inc)
            .with_done_ifs(is_done),
        )?;
    }

    Ok((raw_desc, desc_list))
}

fn extended(
    dma_ch_id: ChannelId,
    src_addr: usize,
    src_addr_inc: AddrInc,
    dst_addr: usize,
    dst_addr_inc: AddrInc,
    unit: UnitSize,
    unit_count: usize,
    is_done: bool,
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
        #[cfg(feature = "debug-spi-dma-defmt-info")]
        info!("\t Loop: {}", &loop_count);

        desc_list = desc_list.push_linked(ImmediateDescriptor::new(
            (loop_count - 1) as u32,
            crate::dma::mmio::dma()
                .ch(dma_ch_id as usize)
                .loop_()
                .as_ptr()
                .addr(),
        ))?;
    }

    desc_list = desc_list.push_linked(
        TransferDescriptor::new(
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
        .with_done_ifs((transfer_count == 1) && is_done),
    )?;

    if transfer_count > 1 {
        #[cfg(feature = "debug-spi-dma-defmt-info")]
        info!("\t transfer_count: {}", &transfer_count);

        if loop_count > 0 {
            desc_list = desc_list.push_linked(
                LoopTransferDescriptor::new(
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

        desc_list = desc_list.push_linked(
            TransferDescriptor::new(
                Addr::Relative(0),
                Addr::Relative(0),
                TransferCount::MAX,
                unit,
            )
            .with_src_inc(src_addr_inc)
            .with_dst_inc(dst_addr_inc)
            .with_done_ifs(is_done),
        )?;
    }

    Ok(desc_list)
}
