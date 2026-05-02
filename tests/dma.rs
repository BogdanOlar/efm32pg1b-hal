#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use cortex_m::asm::nop;
    use defmt::info;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        crc::{algos::CRC_32_CKSUM, Crc, CrcDriver},
        dma::Dma,
        pac::Peripherals,
    };

    struct TestInit {
        dma: Dma,
        crc: Crc<u32>,
    }

    #[init]
    fn init() -> TestInit {
        let p = Peripherals::take().unwrap();
        TestInit {
            dma: Dma::init(p.ldma),
            crc: CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM),
        }
    }

    #[test]
    fn transfer_u8(init: TestInit) {
        let dma = init.dma;
        let crc = init.crc;

        let mut dst_buf: [u8; SRC_U8_SIZE] = [0; _];
        let src = &SRC_U8;
        let dst = &mut dst_buf;

        let mut transfer = dma.ch0.into_transfer(src, dst);

        let transfer_result = loop {
            match transfer.check_done() {
                Some(res) => break res,
                None => {
                    nop();
                }
            }
        };

        assert!(transfer_result.is_ok());

        let (params, copy_count) = transfer_result.unwrap();
        assert_eq!(copy_count, src.len());

        crc.update(src);
        let src_crc = crc.finalize();
        crc.update(params.dst);
        let dst_crc = crc.finalize();

        info!("0x{:X} 0x{:X}", src_crc, dst_crc);

        assert_eq!(src_crc, dst_crc);
    }

    const SRC_U8_SIZE: usize = 1024 * 8;
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
