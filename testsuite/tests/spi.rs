#![no_std]
#![no_main]

use defmt::error;
use efm32pg1b_hal::{crc::Crc, usart::spi};
use embedded_hal::spi::{ErrorType, SpiBus};

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use crate::test_transfer;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        crc::{algos::CRC_32_CKSUM, Crc, CrcDriver},
        dma::descriptor::Descriptor,
        dma::Dma,
        gpio::{Gpio, InFilt, OutPp},
        pac::Peripherals,
        usart::spi::{BitOrder, Config, Spi, SpiPins},
    };
    use embedded_hal::spi::{MODE_0, MODE_2};

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
    fn init() -> (Spi, Crc<u32>, Dma) {
        let p = Peripherals::take().unwrap();
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let gpio = Gpio::new(p.gpio);
        let spi = Spi::new(
            SpiPins::new(
                p.usart0,
                gpio.pc8.into_mode::<OutPp>(),
                gpio.pc6.into_mode::<OutPp>(),
                gpio.pc7.into_mode::<InFilt>(),
            ),
            &Config::new(MODE_2, 1).with_loopback(true),
        );
        let dma = Dma::init(p.ldma);
        (spi, crc, dma)
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_sync((mut spi, crc, _dma): (Spi, Crc<u32>, Dma)) {
        // Set the `dst` length to a multiple of 1
        let mut dst_buf: [u8; Descriptor::MAX_TRANSFER_UNITS] = [0; _];
        let dst_len = dst_buf.len();
        let src = &SRC_BUF;

        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_sync_tx_desc_1_rx_1((mut spi, crc, _dma): (Spi, Crc<u32>, Dma)) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    /// Build the driver with one config, then reconfigure it at runtime via `set_config` and
    /// verify that transfers still succeed.
    #[test]
    #[timeout(5)]
    fn set_config_runtime((mut spi, crc, _dma): (Spi, Crc<u32>, Dma)) {
        let mut dst_buf: [u8; Descriptor::MAX_TRANSFER_UNITS] = [0; _];
        let dst_len = dst_buf.len();
        let src = &SRC_BUF;

        // Initial transfer with the build-time config (MODE_2, loopback enabled)
        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());

        // Reconfigure at runtime: change the SPI mode and baudrate divider, keep loopback enabled.
        // Divider N=8 -> fHFPERCLK/(2*9) = 1.0555 MHz with the default 19 MHz HFRCO.
        let new_cfg = Config::new(MODE_0, 8).with_loopback(true);
        spi.set_config(&new_cfg);

        // Transfer again with the new config; loopback makes TX == RX, so the CRCs must match
        dst_buf.as_mut_slice().fill(0);
        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    /// Verify that the [`BitOrder`] setting is applied from the [`Config`] and that transfers
    /// succeed under both [`BitOrder::MsbFirst`] and [`BitOrder::LsbFirst`].
    ///
    /// In loopback mode the TX and RX shift registers use the same bit order, so the received
    /// byte equals the transmitted byte regardless of `MSBF`. The test therefore confirms that
    /// programming each bit order (both at build time via [`Config::with_bit_order`] and at runtime
    /// via [`Spi::set_config`]) does not break the transfer path.
    #[test]
    #[timeout(5)]
    fn bit_order((mut spi, crc, _dma): (Spi, Crc<u32>, Dma)) {
        let mut dst_buf: [u8; Descriptor::MAX_TRANSFER_UNITS] = [0; _];
        let dst_len = dst_buf.len();
        let src = &SRC_BUF;

        // The driver is built with the default bit order (MsbFirst); a loopback transfer must
        // round-trip the source bytes, so the CRCs must match.
        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());

        // Reconfigure to LSB-first at runtime, keeping the rest of the build-time config
        // (MODE_2, divider 1, loopback enabled). The transfer must still round-trip correctly.
        let lsb_cfg = Config::new(MODE_2, 1)
            .with_loopback(true)
            .with_bit_order(BitOrder::LsbFirst);
        spi.set_config(&lsb_cfg);
        dst_buf.as_mut_slice().fill(0);
        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());

        // Switch back to MSB-first to confirm both directions can be reprogrammed at runtime.
        let msb_cfg = Config::new(MODE_2, 1)
            .with_loopback(true)
            .with_bit_order(BitOrder::MsbFirst);
        spi.set_config(&msb_cfg);
        dst_buf.as_mut_slice().fill(0);
        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());
    }
}

/// Test [`SpiBus`] transfer
///
/// Values of `dst_buf`, `dst_len`, `dst_offset` must conform to:
///
/// ```rs,no_run
///     assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset)
/// ```
fn test_transfer<T: SpiBus + ErrorType>(
    src: &[u8],
    dst_buf: &mut [u8],
    dst_len: usize,
    dst_offset: usize,
    spi: &mut T,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset);
    let dst = &mut dst_buf[dst_offset..dst_offset + dst_len];

    let ret = spi.transfer(dst, src);
    assert!(ret.is_ok());
    let ret = spi.flush();
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
