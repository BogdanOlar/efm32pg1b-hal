#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use defmt::info;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        cmu::CmuExt,
        crc::{algos::CRC_32_CKSUM, CrcDriver},
        gpio::{Gpio, InFilt, OutPp},
        pac::Peripherals,
        usart::{Usart, UsartBuild},
    };
    use embedded_hal::spi::{SpiBus, MODE_2};
    pub use fugit::RateExtU32;

    #[init]
    fn init() -> Peripherals {
        Peripherals::take().unwrap()
    }

    #[test]
    #[timeout(4)]
    fn transfer_u8(p: Peripherals) {
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let clocks = p.cmu.split();
        let gpio = Gpio::new(p.gpio);
        let tx = gpio.pc6.into_mode::<OutPp>();
        let rx = gpio.pc7.into_mode::<InFilt>();
        let clk = gpio.pc8.into_mode::<OutPp>();
        let usart0 = Usart::new(p.usart0);
        let mut spi = usart0.into_spi_bus(clk, tx, rx, MODE_2);
        spi.set_loopback(true);
        let br = spi.set_baudrate(4.MHz(), &clocks);

        // Set the `dst` length to a multiple of 1
        let mut dst_buf: [u8; SRC_U8_SIZE] = [0; _];

        let src = &SRC_U8;
        let dst: &mut [u8; _] = &mut dst_buf;

        let ret_tr1 = spi.transfer(dst, src);
        assert!(ret_tr1.is_ok());

        crc.update(src);
        let src_crc = crc.finalize();
        crc.update(dst);
        let dst_crc = crc.finalize();

        if src_crc != dst_crc {
            info!("BR: {}", br);
            info!("Src: {}", src);
            info!("Dst: {}", dst);
        }

        assert_eq!(src_crc, dst_crc);
    }

    const SRC_U8_SIZE: usize = 1024 * 26;
    #[allow(clippy::large_const_arrays)]
    const SRC_U8: [u8; SRC_U8_SIZE] = {
        let mut seq = [0; SRC_U8_SIZE];
        let mut i = 0;
        // Fill the buffer with values from 1 to 255
        while i < SRC_U8_SIZE {
            seq[i] = 1 + (i % u8::MAX as usize) as u8;
            i += 1;
        }
        seq
    };
}
