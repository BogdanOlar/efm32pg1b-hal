#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {

    use defmt::info;
    use defmt_rtt as _;
    use efm32pg1b_hal::{crc::CrcDriver, pac::Peripherals};

    #[init]
    fn init() -> CrcDriver {
        let p = Peripherals::take().unwrap();
        CrcDriver::new(p.gpcrc)
    }

    /// Test the HAL CRC-16 algos against the `crc` crate algos
    #[test]
    fn crc16(mut driver: CrcDriver) {
        for (lib_algo, hal_algo, algo_name) in CRC16_ALGOS {
            info!("{}", algo_name);

            let lib_crc_algo = crc::Crc::<u16>::new(lib_algo);
            let hal_crc_algo = driver.into_algo_16(hal_algo);

            for s in TEST_DATA {
                let b = s.as_bytes();
                let lib_crc = lib_crc_algo.checksum(b);

                hal_crc_algo.update(b);
                let hal_crc = hal_crc_algo.finalize();
                // info!("\t lib: 0x{:X}\thal: 0x{:X}\t'{}'", lib_crc, hal_crc, s);

                assert_eq!(lib_crc, hal_crc);
            }

            driver = hal_crc_algo.release();
        }
    }

    /// Test the compatibility with the `crc` crate Algorithm definitions
    #[test]
    #[cfg(feature = "crc-lib-compat")]
    fn crc16_compat(mut driver: CrcDriver) {
        for (lib_algo, _, algo_name) in CRC16_ALGOS {
            info!("{}", algo_name);

            let lib_crc_algo = crc::Crc::<u16>::new(lib_algo);
            // Get the HAL algo from the `crc::Algorithm<u16>`
            let hal_crc_algo = driver.into_algo_16(&(*lib_algo).into());

            for s in TEST_DATA {
                let b = s.as_bytes();
                let lib_crc = lib_crc_algo.checksum(b);

                hal_crc_algo.update(b);
                let hal_crc = hal_crc_algo.finalize();
                // info!("\t lib: 0x{:X}\thal: 0x{:X}\t'{}'", lib_crc, hal_crc, s);

                assert_eq!(lib_crc, hal_crc);
            }

            driver = hal_crc_algo.release();
        }
    }

    /// Test the HAL CRC-32 algos against the `crc` crate algos
    #[test]
    fn crc32(mut driver: CrcDriver) {
        for (lib_algo, hal_algo, algo_name) in CRC32_ALGOS {
            info!("{}", algo_name);

            let lib_crc_algo = crc::Crc::<u32>::new(lib_algo);
            let hal_crc_algo = driver.into_algo_32(hal_algo);

            for s in TEST_DATA {
                let b = s.as_bytes();
                let lib_crc = lib_crc_algo.checksum(b);

                hal_crc_algo.update(b);
                let hal_crc = hal_crc_algo.finalize();
                // info!("\t lib: 0x{:X}\thal: 0x{:X} \t '{}'", lib_crc, hal_crc, s);

                assert_eq!(lib_crc, hal_crc);
            }

            driver = hal_crc_algo.release();
        }
    }

    /// Same CRC should be calculated even if this is done in multiple calls to `update()`
    #[test]
    fn split_blocking(driver: CrcDriver) {
        let data = "123456789".as_bytes();

        // CRC-16
        let crc_algo = driver.into_algo_16(&efm32pg1b_hal::crc::algos::CRC_16_ARC);
        crc_algo.update(&data[..data.len() / 2]);
        crc_algo.update(&data[data.len() / 2..]);
        let crc = crc_algo.finalize();
        assert_eq!(crc, 0xbb3d);

        let driver = crc_algo.release();

        // CRC-32
        let crc_algo = driver.into_algo_32(&efm32pg1b_hal::crc::algos::CRC_32_CKSUM);
        for b in data {
            crc_algo.update(&[*b]);
        }
        let crc = crc_algo.finalize();
        assert_eq!(crc, 0x765e7680);
    }

    /// Data width mixing
    #[test]
    fn data_width_blocking(driver: CrcDriver) {
        let crc_algo = driver.into_algo_16(&efm32pg1b_hal::crc::algos::CRC_16_ARC);

        // 10 byte u8 array containing ASCII codes: '0' is `0x30`, '1' is `0x31`, '2' is `0x32`, etc.
        let data1_u8: &[u8] = "0123456789".as_bytes();

        crc_algo.update(data1_u8);
        let crc_u8 = crc_algo.finalize();

        // `data1_u8` is essentially "Big Endian" byte order, so we need to account for that to get the same CRC
        let data2_u32: [u32; 2] = [0x30313233u32.to_be(), 0x34353637u32.to_be()];
        let data2_u16: u16 = 0x3839u16.to_be();

        crc_algo.update(&data2_u32);
        crc_algo.update(&[data2_u16]);
        let crc_mixed = crc_algo.finalize();

        assert_eq!(crc_u8, crc_mixed);
    }

    const TEST_DATA: &[&str] = &[
            "",
            "1",
            "1234",
            "123456789",
            "0123456789ABCDE",
            "01234567890ABCDEFGHIJK",
            "01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK",
        ];

    const CRC16_ALGOS: [(
        &crc::Algorithm<u16>,
        &efm32pg1b_hal::crc::Algorithm<u16>,
        &str,
    ); 31] = [
        (
            &crc::CRC_16_ARC,
            &efm32pg1b_hal::crc::algos::CRC_16_ARC,
            "CRC_16_ARC",
        ),
        (
            &crc::CRC_16_CDMA2000,
            &efm32pg1b_hal::crc::algos::CRC_16_CDMA2000,
            "CRC_16_CDMA2000",
        ),
        (
            &crc::CRC_16_CMS,
            &efm32pg1b_hal::crc::algos::CRC_16_CMS,
            "CRC_16_CMS",
        ),
        (
            &crc::CRC_16_DDS_110,
            &efm32pg1b_hal::crc::algos::CRC_16_DDS_110,
            "CRC_16_DDS_110",
        ),
        (
            &crc::CRC_16_DECT_R,
            &efm32pg1b_hal::crc::algos::CRC_16_DECT_R,
            "CRC_16_DECT_R",
        ),
        (
            &crc::CRC_16_DECT_X,
            &efm32pg1b_hal::crc::algos::CRC_16_DECT_X,
            "CRC_16_DECT_X",
        ),
        (
            &crc::CRC_16_DNP,
            &efm32pg1b_hal::crc::algos::CRC_16_DNP,
            "CRC_16_DNP",
        ),
        (
            &crc::CRC_16_EN_13757,
            &efm32pg1b_hal::crc::algos::CRC_16_EN_13757,
            "CRC_16_EN_13757",
        ),
        (
            &crc::CRC_16_GENIBUS,
            &efm32pg1b_hal::crc::algos::CRC_16_GENIBUS,
            "CRC_16_GENIBUS",
        ),
        (
            &crc::CRC_16_GSM,
            &efm32pg1b_hal::crc::algos::CRC_16_GSM,
            "CRC_16_GSM",
        ),
        (
            &crc::CRC_16_IBM_3740,
            &efm32pg1b_hal::crc::algos::CRC_16_IBM_3740,
            "CRC_16_IBM_3740",
        ),
        (
            &crc::CRC_16_IBM_SDLC,
            &efm32pg1b_hal::crc::algos::CRC_16_IBM_SDLC,
            "CRC_16_IBM_SDLC",
        ),
        (
            &crc::CRC_16_ISO_IEC_14443_3_A,
            &efm32pg1b_hal::crc::algos::CRC_16_ISO_IEC_14443_3_A,
            "CRC_16_ISO_IEC_14443_3_A",
        ),
        (
            &crc::CRC_16_KERMIT,
            &efm32pg1b_hal::crc::algos::CRC_16_KERMIT,
            "CRC_16_KERMIT",
        ),
        (
            &crc::CRC_16_LJ1200,
            &efm32pg1b_hal::crc::algos::CRC_16_LJ1200,
            "CRC_16_LJ1200",
        ),
        (
            &crc::CRC_16_M17,
            &efm32pg1b_hal::crc::algos::CRC_16_M17,
            "CRC_16_M17",
        ),
        (
            &crc::CRC_16_MAXIM_DOW,
            &efm32pg1b_hal::crc::algos::CRC_16_MAXIM_DOW,
            "CRC_16_MAXIM_DOW",
        ),
        (
            &crc::CRC_16_MCRF4XX,
            &efm32pg1b_hal::crc::algos::CRC_16_MCRF4XX,
            "CRC_16_MCRF4XX",
        ),
        (
            &crc::CRC_16_MODBUS,
            &efm32pg1b_hal::crc::algos::CRC_16_MODBUS,
            "CRC_16_MODBUS",
        ),
        (
            &crc::CRC_16_NRSC_5,
            &efm32pg1b_hal::crc::algos::CRC_16_NRSC_5,
            "CRC_16_NRSC_5",
        ),
        (
            &crc::CRC_16_OPENSAFETY_A,
            &efm32pg1b_hal::crc::algos::CRC_16_OPENSAFETY_A,
            "CRC_16_OPENSAFETY_A",
        ),
        (
            &crc::CRC_16_OPENSAFETY_B,
            &efm32pg1b_hal::crc::algos::CRC_16_OPENSAFETY_B,
            "CRC_16_OPENSAFETY_B",
        ),
        (
            &crc::CRC_16_PROFIBUS,
            &efm32pg1b_hal::crc::algos::CRC_16_PROFIBUS,
            "CRC_16_PROFIBUS",
        ),
        (
            &crc::CRC_16_RIELLO,
            &efm32pg1b_hal::crc::algos::CRC_16_RIELLO,
            "CRC_16_RIELLO",
        ),
        (
            &crc::CRC_16_SPI_FUJITSU,
            &efm32pg1b_hal::crc::algos::CRC_16_SPI_FUJITSU,
            "CRC_16_SPI_FUJITSU",
        ),
        (
            &crc::CRC_16_T10_DIF,
            &efm32pg1b_hal::crc::algos::CRC_16_T10_DIF,
            "CRC_16_T10_DIF",
        ),
        (
            &crc::CRC_16_TELEDISK,
            &efm32pg1b_hal::crc::algos::CRC_16_TELEDISK,
            "CRC_16_TELEDISK",
        ),
        (
            &crc::CRC_16_TMS37157,
            &efm32pg1b_hal::crc::algos::CRC_16_TMS37157,
            "CRC_16_TMS37157",
        ),
        (
            &crc::CRC_16_UMTS,
            &efm32pg1b_hal::crc::algos::CRC_16_UMTS,
            "CRC_16_UMTS",
        ),
        (
            &crc::CRC_16_USB,
            &efm32pg1b_hal::crc::algos::CRC_16_USB,
            "CRC_16_USB",
        ),
        (
            &crc::CRC_16_XMODEM,
            &efm32pg1b_hal::crc::algos::CRC_16_XMODEM,
            "CRC_16_XMODEM",
        ),
    ];

    const CRC32_ALGOS: [(
        &crc::Algorithm<u32>,
        &efm32pg1b_hal::crc::Algorithm<u32>,
        &str,
    ); 5] = [
        (
            &crc::CRC_32_BZIP2,
            &efm32pg1b_hal::crc::algos::CRC_32_BZIP2,
            "CRC_32_BZIP2",
        ),
        (
            &crc::CRC_32_CKSUM,
            &efm32pg1b_hal::crc::algos::CRC_32_CKSUM,
            "CRC_32_CKSUM",
        ),
        (
            &crc::CRC_32_ISO_HDLC,
            &efm32pg1b_hal::crc::algos::CRC_32_ISO_HDLC,
            "CRC_32_ISO_HDLC",
        ),
        (
            &crc::CRC_32_JAMCRC,
            &efm32pg1b_hal::crc::algos::CRC_32_JAMCRC,
            "CRC_32_JAMCRC",
        ),
        (
            &crc::CRC_32_MPEG_2,
            &efm32pg1b_hal::crc::algos::CRC_32_MPEG_2,
            "CRC_32_MPEG_2",
        ),
    ];
}
