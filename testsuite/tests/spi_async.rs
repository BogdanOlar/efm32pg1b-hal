#![no_std]
#![no_main]

use defmt::error;
use efm32pg1b_hal::{crc::Crc, usart::spi, usart::spi::dma::SpiDma};

// Provide a defmt timestamp backed by the embassy time driver. The `efemb` feature enables the
// HAL's LeTimer0 time driver, which `#[init]` starts via `Ticker::init()` before any test runs, so
// `Instant::now()` returns real monotonic microseconds from then on (and `0` until then). This
// satisfies the `{t}` placeholder in the probe-rs log format.
defmt::timestamp!("{=u64:us}", {
    embassy_time::Instant::now().as_micros()
});

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use crate::test_transfer_async;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        cmu::{CmuExt, LfClockSource},
        crc::{algos::CRC_32_CKSUM, Crc, CrcDriver},
        dma::descriptor::Descriptor,
        dma::Dma,
        gpio::{Gpio, InFilt, OutPp},
        pac::Peripherals,
        timer_le::efemb::Ticker,
        usart::spi::{Config, SpiPins},
        usart::spi::dma::SpiDma,
    };
    use embedded_hal::spi::MODE_2;

    /// `Descriptor::MAX_TRANSFER_UNITS` = `0x800` = `2048` bytes
    ///
    /// The size of RAM is 32K, and since the tests may use a destination (RX) buffer of size `MAX_RAM_TRANSFERS`, then
    /// the value needs to be smaller than 32K (probably less than that, since the executable also uses some memory)
    const MAX_RAM_TRANSFERS: usize = 13;
    /// This value may need to get adjusted as we add more tests, since it occupies the vast majority of Flash
    const MAX_ROM_TRANSFERS: usize = 80;
    /// Huge ROM (Flash) array
    const SRC_BUF_SIZE: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_ROM_TRANSFERS;

    /// ROM Src bytes with values in repeating interval [1..254]
    static SRC_BUF: [u8; SRC_BUF_SIZE] = {
        let mut seq = [0; SRC_BUF_SIZE];
        let mut i = 0;
        // Fill the buffer with values from 1 to 254 (`0x00` is reserved for the initial contents of the RX buffer,
        // and `0xff` for the filler value for TX transactions where TX is smaller than RX)
        while i < SRC_BUF_SIZE {
            seq[i] = 1 + (i % (u8::MAX as usize - 1)) as u8;
            i += 1;
        }
        seq
    };

    #[init]
    fn init() -> (SpiDma, Crc<u32>) {
        let p = Peripherals::take().unwrap();

        // Configure the clocks required by the embassy time driver. LfAClk must be enabled (here the
        // LFRCO at 32.768 kHz, matching the `efemb-timdrv-letim0-hz-32_768` feature) and the HfClk must
        // come from an HF source so that LeTimer0's `Ticker::init()` doesn't fault. See the warning in
        // [`Ticker::init`].
        let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
        Ticker::init();

        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let gpio = Gpio::new(p.gpio);
        let dma = Dma::init(p.ldma);
        let spi = efm32pg1b_hal::usart::spi::Spi::new(
            SpiPins::new(
                p.usart0,
                gpio.pc8.into_mode::<OutPp>(),
                gpio.pc6.into_mode::<OutPp>(),
                gpio.pc7.into_mode::<InFilt>(),
            ),
            &Config::new(MODE_2, 1).with_loopback(true),
        )
        .into_spi_dma(dma.ch0, dma.ch1);
        (spi, crc)
    }

    /// Async SPI DMA transfer with TX and RX both equal to one descriptor's worth of units.
    ///
    /// Based on the synchronous `transfer_u8_dma_tx_desc_1_rx_desc_1` test in `spi_dma.rs`, but driven
    /// through [`SpiDma::transfer_async`](`efm32pg1b_hal::usart::spi::dma::SpiDma::transfer_async`).
    /// In loopback mode TX == RX, so the received bytes must match the transmitted bytes, which the
    /// CRC comparison in [`test_transfer_async`] verifies.
    #[test]
    #[timeout(5)]
    async fn transfer_async_tx_desc_1_rx_desc_1((mut spi, crc): (SpiDma, Crc<u32>)) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res =
            test_transfer_async(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc).await;
        assert!(test_res.is_ok());
    }
}

/// Test [`SpiDma::transfer_async`] transfer
///
/// Values of `dst_buf`, `dst_len`, `dst_offset` must conform to:
///
/// ```rs,no_run
///     assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset)
/// ```
async fn test_transfer_async(
    src: &[u8],
    dst_buf: &mut [u8],
    dst_len: usize,
    dst_offset: usize,
    spi: &mut SpiDma,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset);
    let dst = &mut dst_buf[dst_offset..dst_offset + dst_len];

    let ret = spi.transfer_async(dst, src).await;
    assert!(ret.is_ok());

    let ret = test_buffers(src, dst_buf, dst_len, dst_offset, crc);
    assert!(ret.is_ok());

    Ok(())
}

/// Test the buffers after the SPI operation has completed
fn test_buffers(
    src: &[u8],
    dst_buf: &[u8],
    dst_len: usize,
    dst_offset: usize,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset);
    let dst = &dst_buf[dst_offset..dst_offset + dst_len];

    // Only compare the common bytes of the two slices `src` and `dst`
    let crc_len = src.len().min(dst.len());
    if crc_len > 0 {
        crc.update(&src[..crc_len]);
        let src_crc = crc.finalize();
        crc.update(&dst[..crc_len]);
        let dst_crc = crc.finalize();

        // src and dest are identical
        if src_crc != dst_crc {
            error!(
                "CRCs don't match! Src: {} bytes, src_crc=0x{:X}, Dst: {} bytes, dst_crc=0x{:X}",
                src.len(),
                src_crc,
                dst.len(),
                dst_crc
            );
            error!("\t dst_buf = {=[?]}", dst_buf);
        }
        assert_eq!(src_crc, dst_crc);
    }

    // Check filler bytes, if they exist
    let dst_filler_len = dst.len().saturating_sub(src.len());
    if dst_filler_len > 0 {
        let start_index = dst.len() - dst_filler_len;
        let end_index = dst.len();

        // filler bytes of `dst`
        for (i, b) in dst[start_index..end_index].iter().enumerate() {
            if *b != spi::TX_FILLER_BYTE {
                error!(
                    "Dst filler: expected 0x{:X}, found 0x{:X}, at index {}",
                    spi::TX_FILLER_BYTE,
                    *b,
                    i + start_index
                );
            }
            assert_eq!(*b, spi::TX_FILLER_BYTE);
        }
    }

    // no bytes written before start of `dst`
    for b in &dst_buf[0..dst_offset] {
        if *b != 0 {
            error!(
                "Dst underflow: expected [0;_], found {=[?]}",
                &dst_buf[0..dst_offset]
            );
        }
        assert_eq!(*b, 0);
    }
    // no bytes written after end of `dst`
    for b in &dst_buf[dst_offset + dst_len..] {
        if *b != 0 {
            error!(
                "Dst overflow: expected [0;_], found {=[?]}",
                &dst_buf[dst_offset + dst_len..]
            );
        }
        assert_eq!(*b, 0);
    }

    Ok(())
}
