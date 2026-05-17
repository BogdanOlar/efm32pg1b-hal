#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use cortex_m::asm::nop;
    use defmt::error;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        cmu::CmuExt,
        crc::{algos::CRC_32_CKSUM, CrcDriver},
        dma::Dma,
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
    #[timeout(5)]
    fn transfer_u8(p: Peripherals) {
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let clocks = p.cmu.split();
        let gpio = Gpio::new(p.gpio);
        let tx = gpio.pc6.into_mode::<OutPp>();
        let rx = gpio.pc7.into_mode::<InFilt>();
        let clk = gpio.pc8.into_mode::<OutPp>();
        let mut spi = Usart::new(p.usart0).into_spi_bus(clk, tx, rx, MODE_2);
        spi.set_loopback(true);
        let rs_br = spi.set_baudrate(4.MHz(), &clocks);
        assert!(rs_br.is_ok());

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

        assert_eq!(src_crc, dst_crc);
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma(p: Peripherals) {
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let clocks = p.cmu.split();
        let gpio = Gpio::new(p.gpio);
        let tx = gpio.pc6.into_mode::<OutPp>();
        let rx = gpio.pc7.into_mode::<InFilt>();
        let clk = gpio.pc8.into_mode::<OutPp>();
        let mut spi = Usart::new(p.usart0).into_spi_bus(clk, tx, rx, MODE_2);
        spi.set_loopback(true);
        let rs_br = spi.set_baudrate(4.MHz(), &clocks);
        assert!(rs_br.is_ok());
        let dma = Dma::init(p.ldma);
        let mut spi = spi.into_spi_dma(dma.ch0, dma.ch1);

        // Set the `dst` length to a multiple of 1
        let mut dst_buf: [u8; SRC_U8_SIZE] = [0; _];

        let src = &SRC_U8;
        // let dst = &mut dst_buf[1..Descriptor::MAX_TRANSFER_UNITS + 1];
        let dst = &mut dst_buf[..10];

        let ret_tr1 = spi.transfer(dst, src);
        assert!(ret_tr1.is_ok());

        // FIXME: DMA transfer still ongoing when CRC is calculated
        for _ in 0..100_000 {
            nop();
        }

        crc.update(&src[..dst.len()]);
        let src_crc = crc.finalize();
        crc.update(dst);
        let dst_crc = crc.finalize();

        // DEBUG:
        if src_crc != dst_crc {
            error!(
                "{} bytes src_crc=0x{:X} dst_crc=0x{:X}",
                src.len().min(dst.len()),
                src_crc,
                dst_crc
            );
            error!("src: {}", src[..dst.len()]);
            error!("dst: {}", dst);
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
