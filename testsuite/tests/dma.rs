#![no_std]
#![no_main]

use defmt::error;
use efm32pg1b_hal::crc::Crc;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use crate::test_transfer;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        crc::{algos::CRC_32_CKSUM, Crc, CrcDriver},
        dma::descriptor::Descriptor,
        dma::Dma,
        pac::Peripherals,
    };

    /// The size of RAM is 32K. The destination buffer and the descriptor list (stored in the tail of `dst`)
    /// both live in RAM, so the total must stay well under 32K.
    const MAX_RAM_TRANSFERS: usize = 13;
    /// Number of units per descriptor (bytes)
    const MTU: usize = Descriptor::MAX_TRANSFER_UNITS;

    // ==== u8 transfers ========================================================================

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_0((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 0;
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 1;
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_desc_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU;
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_desc_2((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU * 2;
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_desc_4((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU * 4;
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u8_dma_desc_max_min_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU * (MAX_RAM_TRANSFERS - 1);
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(15)]
    fn transfer_u8_dma_max_ram((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU * MAX_RAM_TRANSFERS;
        let src = &SRC_BUF_U8[..LEN];
        let mut dst = [0u8; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    // ==== u16 transfers =======================================================================

    #[test]
    #[timeout(5)]
    fn transfer_u16_dma_0((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 0;
        let src = &SRC_BUF_U16[..LEN];
        let mut dst = [0u16; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u16_dma_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 1;
        let src = &SRC_BUF_U16[..LEN];
        let mut dst = [0u16; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u16_dma_desc_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU / 2;
        let src = &SRC_BUF_U16[..LEN];
        let mut dst = [0u16; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u16_dma_desc_2((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU;
        let src = &SRC_BUF_U16[..LEN];
        let mut dst = [0u16; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u16_dma_desc_4((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU * 2;
        let src = &SRC_BUF_U16[..LEN];
        let mut dst = [0u16; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    // ==== u32 transfers =======================================================================

    #[test]
    #[timeout(5)]
    fn transfer_u32_dma_0((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 0;
        let src = &SRC_BUF_U32[..LEN];
        let mut dst = [0u32; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u32_dma_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 1;
        let src = &SRC_BUF_U32[..LEN];
        let mut dst = [0u32; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u32_dma_desc_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU / 4;
        let src = &SRC_BUF_U32[..LEN];
        let mut dst = [0u32; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u32_dma_desc_2((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU / 2;
        let src = &SRC_BUF_U32[..LEN];
        let mut dst = [0u32; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_u32_dma_desc_4((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU;
        let src = &SRC_BUF_U32[..LEN];
        let mut dst = [0u32; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    // ==== struct transfers ====================================================================

    #[test]
    #[timeout(5)]
    fn transfer_struct_4b_dma_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 1;
        let src = &SRC_BUF_SENSOR[..LEN];
        let mut dst = [SensorReading {
            temp: 0,
            humidity: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_4b_dma_128((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 128;
        let src = &SRC_BUF_SENSOR[..LEN];
        let mut dst = [SensorReading {
            temp: 0,
            humidity: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_4b_dma_desc_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU / 4;
        let src = &SRC_BUF_SENSOR[..LEN];
        let mut dst = [SensorReading {
            temp: 0,
            humidity: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_8b_dma_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 1;
        let src = &SRC_BUF_CONFIG[..LEN];
        let mut dst = [ConfigBlock {
            version: 0,
            flags: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_8b_dma_128((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 128;
        let src = &SRC_BUF_CONFIG[..LEN];
        let mut dst = [ConfigBlock {
            version: 0,
            flags: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_8b_dma_desc_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = MTU / 8;
        let src = &SRC_BUF_CONFIG[..LEN];
        let mut dst = [ConfigBlock {
            version: 0,
            flags: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_13b_dma_1((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 1;
        let src = &SRC_BUF_TELEMETRY[..LEN];
        let mut dst = [Telemetry {
            id: 0,
            timestamp: 0,
            value: 0,
            flags: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    #[test]
    #[timeout(5)]
    fn transfer_struct_13b_dma_128((crc, mut dma): (Crc<u32>, Dma)) {
        const LEN: usize = 128;
        let src = &SRC_BUF_TELEMETRY[..LEN];
        let mut dst = [Telemetry {
            id: 0,
            timestamp: 0,
            value: 0,
            flags: 0,
        }; LEN];
        let test_res = test_transfer(&mut dma.ch0, src, &mut dst, &crc);
        assert!(test_res.is_ok());
    }

    // ==== Source buffers (stored in Flash) ====================================================

    const SRC_BUF_U8_SIZE: usize = MTU * MAX_RAM_TRANSFERS;
    static SRC_BUF_U8: [u8; SRC_BUF_U8_SIZE] = {
        let mut seq = [0; SRC_BUF_U8_SIZE];
        let mut i = 0;
        while i < SRC_BUF_U8_SIZE {
            seq[i] = 1 + (i % (u8::MAX as usize - 1)) as u8;
            i += 1;
        }
        seq
    };

    const SRC_BUF_U16_LEN: usize = MTU * MAX_RAM_TRANSFERS / 2;
    static SRC_BUF_U16: [u16; SRC_BUF_U16_LEN] = {
        let mut seq = [0; SRC_BUF_U16_LEN];
        let mut i = 0;
        while i < SRC_BUF_U16_LEN {
            seq[i] = (1 + (i % (u16::MAX as usize - 1)) as u16).to_le();
            i += 1;
        }
        seq
    };

    const SRC_BUF_U32_LEN: usize = MTU * MAX_RAM_TRANSFERS / 4;
    static SRC_BUF_U32: [u32; SRC_BUF_U32_LEN] = {
        let mut seq = [0; SRC_BUF_U32_LEN];
        let mut i = 0;
        while i < SRC_BUF_U32_LEN {
            seq[i] = (1 + (i % (u32::MAX as usize - 1)) as u32).to_le();
            i += 1;
        }
        seq
    };

    /// A small struct (4 bytes) that is `Copy` and has no padding.
    #[derive(Clone, Copy, defmt::Format, PartialEq)]
    struct SensorReading {
        temp: u16,
        humidity: u16,
    }

    /// A medium struct (8 bytes) that is `Copy` and has no padding.
    #[derive(Clone, Copy, defmt::Format, PartialEq)]
    struct ConfigBlock {
        version: u32,
        flags: u32,
    }

    /// An irregularly-sized struct (13 bytes of fields, padded to 16 by alignment).
    #[derive(Clone, Copy, defmt::Format, PartialEq)]
    struct Telemetry {
        id: u32,
        timestamp: u32,
        value: u32,
        flags: u8,
    }

    const SRC_BUF_SENSOR_LEN: usize = MTU * MAX_RAM_TRANSFERS / 4;
    static SRC_BUF_SENSOR: [SensorReading; SRC_BUF_SENSOR_LEN] = {
        let mut seq = [SensorReading {
            temp: 0,
            humidity: 0,
        }; SRC_BUF_SENSOR_LEN];
        let mut i = 0;
        while i < SRC_BUF_SENSOR_LEN {
            seq[i] = SensorReading {
                temp: (1 + (i % (u16::MAX as usize - 1)) as u16).to_le(),
                humidity: (i as u16).wrapping_mul(7).to_le(),
            };
            i += 1;
        }
        seq
    };

    const SRC_BUF_CONFIG_LEN: usize = MTU * MAX_RAM_TRANSFERS / 8;
    static SRC_BUF_CONFIG: [ConfigBlock; SRC_BUF_CONFIG_LEN] = {
        let mut seq = [ConfigBlock {
            version: 0,
            flags: 0,
        }; SRC_BUF_CONFIG_LEN];
        let mut i = 0;
        while i < SRC_BUF_CONFIG_LEN {
            seq[i] = ConfigBlock {
                version: (i as u32).to_le(),
                flags: (i as u32).wrapping_mul(3).to_le(),
            };
            i += 1;
        }
        seq
    };

    const SRC_BUF_TELEMETRY_LEN: usize = MTU * MAX_RAM_TRANSFERS / 16;
    static SRC_BUF_TELEMETRY: [Telemetry; SRC_BUF_TELEMETRY_LEN] = {
        let mut seq = [Telemetry {
            id: 0,
            timestamp: 0,
            value: 0,
            flags: 0,
        }; SRC_BUF_TELEMETRY_LEN];
        let mut i = 0;
        while i < SRC_BUF_TELEMETRY_LEN {
            seq[i] = Telemetry {
                id: (i as u32).to_le(),
                timestamp: (i as u32).wrapping_mul(2).to_le(),
                value: (i as u32).wrapping_mul(3).to_le(),
                flags: (i as u8).wrapping_mul(5),
            };
            i += 1;
        }
        seq
    };

    #[init]
    fn init() -> (Crc<u32>, Dma) {
        let p = Peripherals::take().unwrap();
        let crc = CrcDriver::new(p.gpcrc).into_algo_32(&CRC_32_CKSUM);
        let dma = Dma::init(p.ldma);
        (crc, dma)
    }
}

/// Test a memory-to-memory DMA transfer
///
/// Starts the transfer via `memory_transfer`, waits for completion via `try_resolve`, then
/// compares the CRCs of `src` and `dst`.
fn test_transfer<W: Sized>(
    ch: &mut efm32pg1b_hal::dma::DmaChannel,
    src: &[W],
    dst: &mut [W],
    crc: &Crc<u32>,
) -> Result<(), ()> {
    assert_eq!(src.len(), dst.len());

    let transfer_result = {
        let mut transfer = match ch.memory_transfer(src, dst) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to start DMA transfer: {}", e);
                return Err(());
            }
        };

        loop {
            if let Some(res) = transfer.try_resolve() {
                break res;
            }
        }
    };

    if let Err(e) = transfer_result {
        error!("DMA transfer failed: {}", e);
        return Err(());
    }

    // Compare CRCs of src and dst
    let byte_len = core::mem::size_of_val(src);
    if byte_len > 0 {
        crc.update(src);
        let src_crc = crc.finalize();
        crc.update(dst);
        let dst_crc = crc.finalize();

        if src_crc != dst_crc {
            error!(
                "CRCs don't match! Src: {} bytes, src_crc=0x{:X}, Dst: {} bytes, dst_crc=0x{:X}",
                byte_len, src_crc, byte_len, dst_crc
            );
        }
        assert_eq!(src_crc, dst_crc);
    }

    Ok(())
}
