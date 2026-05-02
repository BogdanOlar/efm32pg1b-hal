#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use defmt::info;
    use defmt_rtt as _;
    use efm32pg1b_hal::{crc::Crc, pac::Peripherals};

    const TEST_DATA: &[&str] = &[
            "",
            "1",
            "1234",
            "123456789",
            "0123456789ABCDE",
            "01234567890ABCDEFGHIJK",
            "01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK01234567890ABCDEFGHIJK",
        ];

    #[init]
    fn init() -> Crc {
        let p = Peripherals::take().unwrap();
        Crc::new(p.gpcrc)
    }

    #[test]
    fn takes_state(driver: Crc) {
        const X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
        let crc_algo = driver.into_algo_16(&efm32pg1b_hal::crc::CRC_16_IBM_SDLC);

        for (i, s) in TEST_DATA.iter().enumerate() {
            let b = s.as_bytes();
            let lib_crc = X25.checksum(b);

            crc_algo.update(b);
            let hal_crc = crc_algo.finalize();
            info!("{}\t lib: 0x{:X} \t hal: 0x{:X}", i, lib_crc, hal_crc,);

            assert_eq!(lib_crc, hal_crc);
        }
    }
}
