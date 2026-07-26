//! SPI bus with DMA transfers

use crate::{
    dma::{
        self,
        descriptor::{
            Addr, AddrInc, Descriptor, ImmediateDescBuilder, LoopTransferDescBuilder,
            TransferCount, TransferDescBuilder, UnitSize,
        },
        ChReqSel, DmaChannel,
    },
    usart::{spi::SpiError, usarts::usartx},
};
use cortex_m::asm;
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

        let tx_units = write.len() / unit.bytes();
        // assert_eq!(write.len() % unit.bytes(), 0);
        let rx_units = read.len() / unit.bytes();
        // assert_eq!(read.len() % unit.bytes(), 0);
        let tx_filler_units = rx_units.saturating_sub(tx_units);
        let transfer_units = tx_units + tx_filler_units;
        assert_eq!(transfer_units, rx_units.max(tx_units));

        if self.busy {
            // FIXME: [`embedded_hal::spi::SpiBus`] docs disallow returning a `Busy` error, though it's not clear to me
            //        what the implementation should do. Enqueueing the request is problematic because if one of the
            //        queued transfer fails, then how is the user supposed to know which one failed?
            Err(SpiError::Busy)
        } else if transfer_units == 0 {
            // FIXME: make sure the tx DMA channel gets its "done" token
            Ok(())
        } else {
            let usart_p = usartx::<N>();

            // TX
            {
                let first_desc_units = if tx_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
                    Descriptor::MAX_TRANSFER_UNITS
                } else {
                    tx_units % Descriptor::MAX_TRANSFER_UNITS
                };

                let mut first_desc_builder = unsafe {
                    TransferDescBuilder::new(
                        Addr::Absolute(write.as_ptr().addr()),
                        Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                        first_desc_units.try_into().unwrap(),
                        unit,
                    )
                }
                .with_dst_inc(AddrInc::None);

                // if there are more units to TX (including filler units), then use the TX descriptor linked list
                if transfer_units - first_desc_units > 0 {
                    first_desc_builder = first_desc_builder
                        .with_link(Addr::Absolute(self.tx_descriptors.as_ptr().addr()));
                } else {
                    // this is the only descriptor, enable DONE Interrupt Flag Set
                    first_desc_builder = first_desc_builder.with_done_ifs();
                }

                let mut desc_list = self.tx_descriptors.iter_mut();

                let tx_rem_units = tx_units - first_desc_units;
                assert!(tx_rem_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS));

                let mut tx_loops = tx_rem_units / Descriptor::MAX_TRANSFER_UNITS;
                let mut cur_addr = write.as_ptr().addr() + (first_desc_units * unit.bytes());

                while tx_loops > 0 {
                    if tx_loops < 2 {
                        let desc_loops = tx_loops.min(u8::MAX as usize);
                        *desc_list.next().unwrap() = unsafe {
                            TransferDescBuilder::new(
                                Addr::Absolute(cur_addr),
                                Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                                TransferCount::MAX,
                                unit,
                            )
                        }
                        .with_dst_inc(AddrInc::None)
                        .with_done_ifs()
                        .build();

                        tx_loops -= desc_loops;
                        cur_addr += desc_loops * Descriptor::MAX_TRANSFER_UNITS * unit.bytes();
                    } else {
                        // We're going to use one descriptor with absolute address in order to restore the `src` and
                        // `dst` registers of the DMA channel peripheral
                        tx_loops -= 1;

                        let desc_loops = tx_loops.min(u8::MAX as usize);

                        *desc_list.next().unwrap() = unsafe {
                            ImmediateDescBuilder::new(
                                desc_loops as u32,
                                crate::dma::mmio::dma()
                                    .ch(self.tx.id() as usize)
                                    .loop_()
                                    .as_ptr()
                                    .addr(),
                            )
                        }
                        // move on to the transfer descriptor (below) after writing the loop count
                        .with_link(Addr::Relative(1))
                        .build();
                        tx_loops -= desc_loops;

                        *desc_list.next().unwrap() = unsafe {
                            TransferDescBuilder::new(
                                Addr::Absolute(cur_addr),
                                Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                                TransferCount::MAX,
                                unit,
                            )
                        }
                        .with_dst_inc(AddrInc::None)
                        .with_link(Addr::Relative(1))
                        .build();
                        cur_addr += Descriptor::MAX_TRANSFER_UNITS * unit.bytes();

                        let mut looped_transfer_desc = LoopTransferDescBuilder::new(
                            Addr::Relative(0),
                            Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                            TransferCount::MAX,
                            unit,
                        )
                        // we're writing to the TXDATA SPI register, so don't increment destination address
                        .with_dst_inc(AddrInc::None)
                        // this is a looped transfer descriptor (the link is also set below)
                        .with_loop(Addr::Relative(0));

                        if tx_loops == 0 && tx_filler_units == 0 {
                            // this is the last descriptor and there are no filler bytes following, so set the ISR flag
                            looped_transfer_desc =
                                looped_transfer_desc.with_done_ifs().with_link(false)
                        } else {
                            // there are additional units to TX, so there will be more descriptors following this one,
                            // once the loop counter reaches 0
                            looped_transfer_desc = looped_transfer_desc.with_link(true)
                        }
                        *desc_list.next().unwrap() = looped_transfer_desc.build();
                        cur_addr += desc_loops * Descriptor::MAX_TRANSFER_UNITS * unit.bytes();
                    }
                }

                if tx_filler_units > 0 {
                    let first_filler_desc_units =
                        if tx_filler_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
                            Descriptor::MAX_TRANSFER_UNITS
                        } else {
                            tx_filler_units % Descriptor::MAX_TRANSFER_UNITS
                        };
                    let first_filler_desc_builder = unsafe {
                        TransferDescBuilder::new(
                            Addr::Absolute((&TX_FILLER as *const u32).addr()),
                            Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                            first_filler_desc_units.try_into().unwrap(),
                            unit,
                        )
                    }
                    .with_src_inc(AddrInc::None)
                    .with_dst_inc(AddrInc::None);

                    let remaining_filler_units = tx_filler_units - first_filler_desc_units;
                    assert!(remaining_filler_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS));

                    if remaining_filler_units == 0 {
                        *desc_list.next().unwrap() =
                            first_filler_desc_builder.with_done_ifs().build();
                    } else {
                        *desc_list.next().unwrap() = first_filler_desc_builder
                            .with_link(Addr::Relative(1))
                            .build();

                        let mut filler_loops =
                            remaining_filler_units / Descriptor::MAX_TRANSFER_UNITS;

                        while filler_loops > 0 {
                            let desc_loops = filler_loops.min(u8::MAX as usize);

                            let loop_writer_desc = unsafe {
                                ImmediateDescBuilder::new(
                                    desc_loops as u32,
                                    crate::dma::mmio::dma()
                                        .ch(self.tx.id() as usize)
                                        .loop_()
                                        .as_ptr()
                                        .addr(),
                                )
                            }
                            // move on to the transfer descriptor (below) after writing the loop count
                            .with_link(Addr::Relative(1));
                            *desc_list.next().unwrap() = loop_writer_desc.build();

                            let mut looped_transfer_desc = LoopTransferDescBuilder::new(
                                Addr::Absolute((&TX_FILLER as *const u32).addr()),
                                Addr::Absolute(usart_p.txdata().as_ptr().addr()),
                                TransferCount::MAX,
                                unit,
                            )
                            .with_loop(Addr::Relative(0))
                            // we're reading from the FILLER, so don't increment
                            // we're writing to the TXDATA SPI register, so don't increment destination address
                            .with_src_inc(AddrInc::None)
                            .with_dst_inc(AddrInc::None);

                            // update `filler_loops` here so that we can know if there will be more descriptors after
                            // this one
                            filler_loops -= desc_loops;

                            if filler_loops == 0 {
                                // this is the last descriptor, so set the ISR flag
                                looped_transfer_desc =
                                    looped_transfer_desc.with_done_ifs().with_link(false)
                            } else {
                                // there are additional units to TX, so there will be more descriptors following this
                                // one, once the loop counter reaches 0
                                looped_transfer_desc = looped_transfer_desc.with_link(true)
                            }

                            *desc_list.next().unwrap() = looped_transfer_desc.build();
                        }
                    }
                }

                // make sure all linked descriptors have been written before proceeding
                asm::dsb();

                // write the first TX descriptor to the DMA registers
                self.tx.set_descriptor(&first_desc_builder.build());
            }

            // RX
            {
                let first_desc_units = if rx_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
                    Descriptor::MAX_TRANSFER_UNITS
                } else {
                    rx_units % Descriptor::MAX_TRANSFER_UNITS
                };
                let mut cur_addr = read.as_ptr().addr();

                let mut first_desc_builder = TransferDescBuilder::new(
                    Addr::Absolute(usart_p.rxdata().as_ptr().addr()),
                    Addr::Absolute(read.as_ptr().addr()),
                    first_desc_units.try_into().unwrap(),
                    unit,
                )
                .with_src_inc(AddrInc::None)
                .with_dst_inc(AddrInc::One);
                cur_addr += first_desc_units * unit.bytes();

                let rx_rem_units = rx_units - first_desc_units;
                assert!(rx_rem_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS));

                if rx_rem_units > 0 {
                    // if there are more units to RX then use the RX descriptor linked list
                    first_desc_builder = first_desc_builder
                        .with_link(Addr::Absolute(self.rx_descriptors.as_ptr().addr()));
                } else {
                    // this is the only descriptor, enable DONE Interrupt Flag Set
                    first_desc_builder = first_desc_builder.with_done_ifs();
                }

                let mut desc_list = self.rx_descriptors.iter_mut();

                let mut rx_loops = rx_rem_units / Descriptor::MAX_TRANSFER_UNITS;

                while rx_loops > 0 {
                    if rx_loops == 1 {
                        *desc_list.next().unwrap() = TransferDescBuilder::new(
                            Addr::Absolute(usart_p.rxdata().as_ptr().addr()),
                            Addr::Relative(0),
                            TransferCount::MAX,
                            unit,
                        )
                        // we're reading from the RXDATA SPI register, so don't increment destination address
                        .with_src_inc(AddrInc::None)
                        .with_done_ifs()
                        .build();

                        cur_addr += Descriptor::MAX_TRANSFER_UNITS * unit.bytes();
                        rx_loops -= 1;
                    } else {
                        // for the absolute addr transfer descriptor
                        rx_loops -= 1;
                        let desc_loops = rx_loops.min(u8::MAX as usize);

                        *desc_list.next().unwrap() = unsafe {
                            ImmediateDescBuilder::new(
                                desc_loops as u32,
                                crate::dma::mmio::dma()
                                    .ch(self.rx.id() as usize)
                                    .loop_()
                                    .as_ptr()
                                    .addr(),
                            )
                        }
                        .with_link(Addr::Relative(1))
                        .build();

                        let mut abs_transfer_desc = TransferDescBuilder::new(
                            Addr::Absolute(usart_p.rxdata().as_ptr().addr()),
                            Addr::Absolute(cur_addr),
                            TransferCount::MAX,
                            unit,
                        )
                        .with_src_inc(AddrInc::None)
                        .with_link(Addr::Relative(1));

                        *desc_list.next().unwrap() = abs_transfer_desc.build();
                        cur_addr += Descriptor::MAX_TRANSFER_UNITS * unit.bytes();

                        let mut looped_transfer_desc = LoopTransferDescBuilder::new(
                            Addr::Absolute(usart_p.rxdata().as_ptr().addr()),
                            Addr::Relative(0),
                            TransferCount::MAX,
                            unit,
                        )
                        // we're reading from the RXDATA SPI register, so don't increment destination address
                        .with_src_inc(AddrInc::None)
                        // this is a looped transfer descriptor (the link is also set below)
                        .with_loop(Addr::Relative(0));
                        cur_addr += desc_loops * Descriptor::MAX_TRANSFER_UNITS * unit.bytes();
                        rx_loops -= desc_loops;

                        if rx_loops == 0 {
                            // this is the last descriptor and there are no filler bytes following, so set the ISR flag
                            looped_transfer_desc =
                                looped_transfer_desc.with_done_ifs().with_link(false)
                        } else {
                            // there are additional units to TX, so there will be more descriptors following this one,
                            // once the loop counter reaches 0
                            looped_transfer_desc = looped_transfer_desc.with_link(true)
                        }

                        *desc_list.next().unwrap() = looped_transfer_desc.build();
                    }
                }

                // make sure all linked descriptors have been written before proceeding
                asm::dsb();

                // write the first RX descriptor to the DMA registers
                self.rx.set_descriptor(&first_desc_builder.build());
            }

            // start the transfer
            self.busy = true;
            self.rx.set_ien();
            self.rx.set_enable();
            self.tx.set_ien();
            self.tx.set_enable();
            self.tx.start();

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
