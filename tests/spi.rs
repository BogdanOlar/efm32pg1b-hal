#![no_std]
#![no_main]

use defmt::error;
use efm32pg1b_hal::{
    cmu::CmuExt,
    crc::{algos::CRC_32_CKSUM, Crc, CrcDriver},
    dma::Dma,
    gpio::{Gpio, InFilt, OutPp},
    pac::Peripherals,
    usart::{spi::dma::SpiDma, Usart, UsartBuild},
};
use embedded_hal::spi::{ErrorType, SpiBus, MODE_2};
pub use fugit::RateExtU32;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use crate::{build_drivers, test_read, test_transfer, test_write};
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        cmu::CmuExt,
        crc::{algos::CRC_32_CKSUM, CrcDriver},
        dma::descriptor::Descriptor,
        gpio::{Gpio, InFilt, OutPp},
        pac::Peripherals,
        usart::{Usart, UsartBuild},
    };
    use embedded_hal::spi::MODE_2;
    pub use fugit::RateExtU32;

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
    fn init() -> Peripherals {
        Peripherals::take().unwrap()
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_sync(p: Peripherals) {
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let clocks = p.cmu.split();
        let gpio = Gpio::new(p.gpio);
        let mut spi = Usart::new(p.usart0)
            .into_spi_bus(
                gpio.pc8.into_mode::<OutPp>(),
                gpio.pc6.into_mode::<OutPp>(),
                gpio.pc7.into_mode::<InFilt>(),
                MODE_2,
            )
            .with_loopback();
        let rs_br = spi.set_baudrate(4.MHz(), &clocks);
        assert!(rs_br.is_ok());

        // Set the `dst` length to a multiple of 1
        let mut dst_buf: [u8; Descriptor::MAX_TRANSFER_UNITS] = [0; _];
        let dst_len = dst_buf.len();
        let src = &SRC_BUF;

        let test_res = test_transfer(src, &mut dst_buf, dst_len, 0, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_0_rx_0(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_0_rx_desc_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 1;
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_sync_tx_desc_1_rx_1(p: Peripherals) {
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let clocks = p.cmu.split();
        let gpio = Gpio::new(p.gpio);
        let mut spi = Usart::new(p.usart0)
            .into_spi_bus(
                gpio.pc8.into_mode::<OutPp>(),
                gpio.pc6.into_mode::<OutPp>(),
                gpio.pc7.into_mode::<InFilt>(),
                MODE_2,
            )
            .with_loopback();
        let rs_br = spi.set_baudrate(4.MHz(), &clocks);
        assert!(rs_br.is_ok());

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

        for i in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());

            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_1_rx_desc_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_1_rx_0(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_1_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 1;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_2_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_2(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_3_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_4_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_4(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_max_min_1_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_max_min_1(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    /// SPI transfer uses more than one descriptor for both TX and RX
    /// TX and RX slices have the same size
    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_max_ram(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 0;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_0(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_2(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_4(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_max_min_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_max_ram(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_0(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 1;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_2(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_4(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_max_min_1(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_max_ram(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_max_rom(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_ROM_TRANSFERS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_0_rx_0_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_0_rx_desc_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 1;
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_1_rx_desc_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_1_rx_0_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_1_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 1;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_2_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_2_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_3_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_3_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_4_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_1_rx_desc_4_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_tx_desc_max_min_1_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);
        // RX
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(15)]
    fn transfer_u8_dma_tx_1_rx_desc_max_min_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        // TX
        const SRC_LEN: usize = 1;
        // RX
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(15)]
    fn transfer_u8_dma_max_ram_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 0;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res =
                test_transfer(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_0_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 1;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_2_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_3_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn read_u8_dma_rx_desc_4_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(15)]
    fn read_u8_dma_rx_desc_max_min_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(15)]
    fn read_u8_dma_rx_desc_max_ram_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_read(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_0_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 0;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = 1;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_2_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 2;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_3_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 3;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_4_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * 4;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(5)]
    fn write_u8_dma_tx_desc_max_min_1_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1);
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(15)]
    fn write_u8_dma_tx_desc_max_ram_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }

    #[test]
    #[timeout(15)]
    fn write_u8_dma_tx_desc_max_rom_mul_3(p: Peripherals) {
        // Size of slices which will be tested
        const SRC_LEN: usize = Descriptor::MAX_TRANSFER_UNITS * MAX_ROM_TRANSFERS;
        const DST_LEN: usize = 0;

        // Total size of the destination buffer, including any before+after padding, which are used to test
        // under/overflow
        const DST_BUF_OFFSET: usize = 10;
        const DST_BUF_SIZE: usize = DST_BUF_OFFSET + DST_LEN + DST_BUF_OFFSET;

        let (mut spi, crc) = build_drivers(p);

        let src = &SRC_BUF[..SRC_LEN];
        let mut dst_buf: [u8; DST_BUF_SIZE] = [0; _];

        for _ in 0..3 {
            let test_res = test_write(src, &mut dst_buf, DST_LEN, DST_BUF_OFFSET, &mut spi, &crc);
            assert!(test_res.is_ok());
            // reset buffer
            dst_buf.as_mut_slice().fill(u8::default());
        }
    }
}

/// Helper function to build the SPI DMA and Crc drivers to be used by tests
///
/// Cannot be defined in the `mod tests` above since `embedded-test` will freak out
fn build_drivers(p: Peripherals) -> (SpiDma<0>, Crc<u32>) {
    let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
    let clocks = p.cmu.split();
    let gpio = Gpio::new(p.gpio);
    let mut spi = Usart::new(p.usart0)
        .into_spi_bus(
            gpio.pc8.into_mode::<OutPp>(),
            gpio.pc6.into_mode::<OutPp>(),
            gpio.pc7.into_mode::<InFilt>(),
            MODE_2,
        )
        .with_loopback();

    let rs_br = spi.set_baudrate(4.MHz(), &clocks);
    assert!(rs_br.is_ok());
    let dma = Dma::init(p.ldma);
    let spi = spi.into_spi_dma(dma.ch0, dma.ch1);

    (spi, crc)
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

/// Test [`SpiBus`] read
///
/// Values of `dst_buf`, `dst_len`, `dst_offset` must conform to:
///
/// ```rs,no_run
///     assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset)
/// ```
fn test_read<T: SpiBus + ErrorType>(
    src: &[u8],
    dst_buf: &mut [u8],
    dst_len: usize,
    dst_offset: usize,
    spi: &mut T,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset);
    let dst = &mut dst_buf[dst_offset..dst_offset + dst_len];

    let ret = spi.read(dst);
    assert!(ret.is_ok());
    let ret = spi.flush();
    assert!(ret.is_ok());

    let ret = test_buffers(src, dst_buf, dst_len, dst_offset, crc);
    assert!(ret.is_ok());

    Ok(())
}

/// Test [`SpiBus`] write
///
fn test_write<T: SpiBus + ErrorType>(
    src: &[u8],
    dst_buf: &mut [u8],
    dst_len: usize,
    dst_offset: usize,
    spi: &mut T,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    let ret = spi.write(src);
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
    const FILLER_VALUE: u8 = 0xFF;

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
            if *b != FILLER_VALUE {
                error!(
                    "Dst filler: expected 0x{:X}, found 0x{:X}, at index {}",
                    FILLER_VALUE,
                    *b,
                    i + start_index
                );
            }
            assert_eq!(*b, FILLER_VALUE);
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
