#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use core::cmp::min;

    use cortex_m::asm::nop;
    use defmt::error;
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

    /// make sure the copy is actually doable
    #[test]
    #[timeout(2)]
    fn simple_copy_u8(init: TestInit) {
        let crc = init.crc;

        let mut dst_buf: [u8; SRC_U8_SIZE] = [0; _];
        let src = &SRC_U8;
        let dst = &mut dst_buf;

        dst.copy_from_slice(src);

        crc.update(src);
        let src_crc = crc.finalize();
        crc.update(dst);
        let dst_crc = crc.finalize();

        assert_eq!(src_crc, dst_crc);
    }

    #[test]
    #[timeout(2)]
    fn transfer_u8(init: TestInit) {
        let dma = init.dma;
        let crc = init.crc;

        // Set the `dst` length to a multiple of 1
        let mut dst_buf: [u8; SRC_U8_SIZE - 1] = [0; _];
        let src = &SRC_U8;
        let dst: &mut [u8; _] = &mut dst_buf;

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
        let _ch = params.ch;
        let src = params.src;
        let dst = params.dst;
        assert_eq!(copy_count, min(src.len(), dst.len()));

        crc.update(&src[..copy_count]);
        let src_crc = crc.finalize();
        crc.update(&dst[..copy_count]);
        let dst_crc = crc.finalize();

        // DEBUG
        if src_crc != dst_crc {
            error!(
                "{} bytes (0x{:X}), with unit {}",
                core::mem::size_of_val(dst),
                core::mem::size_of_val(dst),
                transfer.unit()
            );
            error!("src: {}", src[src.len() - 100..]);
            error!("dst: {}", dst[dst.len() - 100..]);
        }

        assert_eq!(src_crc, dst_crc);
    }

    #[test]
    #[timeout(2)]
    fn transfer_u16(init: TestInit) {
        let dma = init.dma;
        let crc = init.crc;

        // Set the `dst` length to a multiple of 2
        let mut dst_buf: [u8; SRC_U8_SIZE - 2] = [0; _];
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
        let _ch = params.ch;
        let src = params.src;
        let dst = params.dst;
        assert_eq!(copy_count, min(src.len(), dst.len()));

        crc.update(&src[..copy_count]);
        let src_crc = crc.finalize();
        crc.update(&dst[..copy_count]);
        let dst_crc = crc.finalize();

        assert_eq!(src_crc, dst_crc);
    }

    #[test]
    #[timeout(2)]
    fn transfer_u32(init: TestInit) {
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
        assert_eq!(src.len(), params.dst.len());

        let _ch = params.ch;
        let src = params.src;
        let dst = params.dst;

        crc.update(&src[..copy_count]);
        let src_crc = crc.finalize();
        crc.update(&dst[..copy_count]);
        let dst_crc = crc.finalize();

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
