#![no_std]
#![no_main]

use defmt::error;
use efm32pg1b_hal::{crc::Crc, usart::spi, usart::spi::dma::SpiDma};

// Provide a defmt timestamp backed by the embassy time driver. The `efemb` feature enables the
// HAL's LeTimer0 time driver, which `#[init]` starts via `Ticker::init()` before any test runs, so
// `Instant::now()` returns real monotonic microseconds from then on (and `0` until then). This
// satisfies the `{t}` placeholder in the probe-rs log format.
defmt::timestamp!("{=u64:us}", { embassy_time::Instant::now().as_micros() });

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use crate::{test_read_async, test_transfer_async, test_write_async};
    use defmt::error;
    use defmt_rtt as _;
    use efm32pg1b_hal::{
        cmu::{CmuExt, LfClockSource},
        crc::{algos::CRC_32_CKSUM, Crc, CrcDriver},
        dma::{descriptor::Descriptor, Dma},
        gpio::{Gpio, InFilt, OutPp},
        pac::Peripherals,
        timer_le::efemb::Ticker,
        usart::spi::{dma::SpiDma, Config, SpiPins},
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

    /// Per-test destination-buffer padding (matches `spi_dma.rs`).
    const DST_BUF_OFFSET: usize = 10;

    /// Shared destination (RX) buffer, sized for the largest case (`read` `rx_desc_max_ram`:
    /// `DST_BUF_OFFSET + MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS + DST_BUF_OFFSET`).
    ///
    /// Async tests are spawned by `embedded_test` as individual embassy tasks, so each test's
    /// future lives in a static `TaskStorage`. A per-test stack `dst_buf` would place every test's
    /// buffer in its own static and overflow RAM (the `spi_dma.rs` sync tests avoid this because
    /// their stack buffers are reused one at a time). Instead all cases share this single buffer;
    /// only one `#[test]` runs per binary invocation, so the access is exclusive.
    const DST_BUF_SIZE: usize =
        2 * DST_BUF_OFFSET + Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS;
    static mut DST_BUF: [u8; DST_BUF_SIZE] = [0u8; DST_BUF_SIZE];

    /// Borrow a zeroed, `'static` slice of the shared [`DST_BUF`] of length `len`.
    ///
    /// Sound because only one async test runs per binary invocation (the `embedded_test` harness
    /// runs a single `#[test]` per process), so the returned `&mut` is always exclusive, and the
    /// DMA completion IRQ never touches this buffer. The `'static` lifetime is required so the
    /// slice can be held across `.await` points within the embassy task.
    fn dst_buf(len: usize) -> &'static mut [u8] {
        // Go through a raw pointer to avoid borrowing a `static mut` directly (denied by the
        // `static_mut_refs` lint).
        let slice: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(DST_BUF) as *mut u8, len)
        };
        slice.fill(0);
        slice
    }

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

    /// A full-duplex SPI test case adapted from `spi_dma.rs`: `(src_len, dst_len, offset, repeat)`.
    ///
    /// Both TX (MOSI) and RX (MISO) are active. `repeat == 3` reproduces the `mul_3` tests
    /// (3 iterations with the destination buffer reset between them).
    #[derive(Clone, Copy)]
    struct FullDuplexCase {
        /// TX (source) slice length, in bytes.
        src_len: usize,
        /// RX (destination) slice length, in bytes.
        dst_len: usize,
        /// Padding before and after the destination slice, used to detect under/overflow.
        offset: usize,
        /// Number of iterations (`1` for the single tests, `3` for the `mul_3` tests).
        repeat: u8,
    }

    /// A simplex SPI test case adapted from `spi_dma.rs`: `(len, offset, repeat)`.
    ///
    /// Only one direction is active: RX for `read` (TX is empty) or TX for `write` (RX is empty).
    /// `repeat == 3` reproduces the `mul_3` tests (3 iterations with the destination buffer
    /// reset between them).
    #[derive(Clone, Copy)]
    struct SimplexCase {
        /// Length of the active direction's slice, in bytes (RX for `read`, TX for `write`).
        len: usize,
        /// Padding before and after the destination slice, used to detect under/overflow.
        offset: usize,
        /// Number of iterations (`1` for the single tests, `3` for the `mul_3` tests).
        repeat: u8,
    }

    const TRANSFER_CASES: &[FullDuplexCase] = &[
        FullDuplexCase {
            src_len: 0,
            dst_len: 0,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 0,
            dst_len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: 1,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS,
            dst_len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS,
            dst_len: 0,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS,
            dst_len: 1,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * 2,
            dst_len: 1,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * 2,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * 3,
            dst_len: 1,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * 3,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * 4,
            dst_len: 1,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * 4,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            dst_len: 1,
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            offset: 10,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            offset: 0,
            repeat: 1,
        },
        FullDuplexCase {
            src_len: 0,
            dst_len: 0,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 0,
            dst_len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: 1,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS,
            dst_len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS,
            dst_len: 0,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS,
            dst_len: 1,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * 2,
            dst_len: 1,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * 2,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * 3,
            dst_len: 1,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * 3,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * 4,
            dst_len: 1,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * 4,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            dst_len: 1,
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: 1,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            offset: 10,
            repeat: 3,
        },
        FullDuplexCase {
            src_len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            dst_len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            offset: 0,
            repeat: 3,
        },
    ];

    const READ_CASES: &[SimplexCase] = &[
        SimplexCase {
            len: 0,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: 1,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 2,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 3,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 4,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: 0,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: 1,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 2,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 3,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 4,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            offset: 10,
            repeat: 3,
        },
    ];

    const WRITE_CASES: &[SimplexCase] = &[
        SimplexCase {
            len: 0,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: 1,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 2,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 3,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 4,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * MAX_ROM_TRANSFERS,
            offset: 10,
            repeat: 1,
        },
        SimplexCase {
            len: 0,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: 1,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 2,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 3,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * 4,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * (MAX_RAM_TRANSFERS - 1),
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * MAX_RAM_TRANSFERS,
            offset: 10,
            repeat: 3,
        },
        SimplexCase {
            len: Descriptor::MAX_TRANSFER_UNITS * MAX_ROM_TRANSFERS,
            offset: 10,
            repeat: 3,
        },
    ];

    /// Async SPI DMA `transfer` over every case from `spi_dma.rs` (single + `mul_3`).
    ///
    /// Each entry in [`TRANSFER_CASES`] is `(src_len, dst_len, offset, repeat)`; `repeat == 3`
    /// reproduces the `mul_3` tests (3 iterations with the destination buffer reset between them).
    /// The destination buffer is the shared [`DST_BUF`] (see [`dst_buf`]); in loopback mode the
    /// received bytes must match the transmitted bytes, verified by CRC in [`test_transfer_async`].
    ///
    /// Every case is run to completion: a failed case is logged with its index and parameters and
    /// the harness moves on to the next one. Returns `Ok(())` only if all cases passed.
    #[test]
    #[timeout(60)]
    async fn transfer_async((mut spi, crc): (SpiDma, Crc<u32>)) -> Result<(), ()> {
        let mut failed: usize = 0;
        for (
            i,
            &FullDuplexCase {
                src_len,
                dst_len,
                offset,
                repeat,
            },
        ) in TRANSFER_CASES.iter().enumerate()
        {
            let dst_buf_size = offset + dst_len + offset;
            for r in 0..repeat {
                let src = &SRC_BUF[..src_len];
                let dst = dst_buf(dst_buf_size);
                let res = test_transfer_async(src, dst, dst_len, offset, &mut spi, &crc).await;
                if res.is_err() {
                    error!(
                        "transfer_async: case #{} (iter {}/{}) FAILED — src_len={}, dst_len={}, offset={}",
                        i,
                        r + 1,
                        repeat,
                        src_len,
                        dst_len,
                        offset
                    );
                    failed += 1;
                }
            }
        }
        if failed == 0 {
            Ok(())
        } else {
            error!("transfer_async: {} iteration(s) failed", failed);
            Err(())
        }
    }

    /// Async SPI DMA `read` (RX-only) over every case from `spi_dma.rs` (single + `mul_3`).
    ///
    /// Each entry in [`READ_CASES`] is `(len, offset, repeat)`, where `len` is the RX length
    /// (TX is empty). `repeat == 3` reproduces the `mul_3` tests. The received bytes are the TX
    /// filler (`spi::TX_FILLER_BYTE`) since TX is empty; [`test_read_async`] verifies the buffers.
    ///
    /// Every case is run to completion: a failed case is logged with its index and parameters and
    /// the harness moves on to the next one. Returns `Ok(())` only if all cases passed.
    #[test]
    #[timeout(60)]
    async fn read_async((mut spi, crc): (SpiDma, Crc<u32>)) -> Result<(), ()> {
        let mut failed: usize = 0;
        for (
            i,
            &SimplexCase {
                len,
                offset,
                repeat,
            },
        ) in READ_CASES.iter().enumerate()
        {
            let dst_buf_size = offset + len + offset;
            for r in 0..repeat {
                let dst = dst_buf(dst_buf_size);
                let res = test_read_async(&[], dst, len, offset, &mut spi, &crc).await;
                if res.is_err() {
                    error!(
                        "read_async: case #{} (iter {}/{}) FAILED — len={}, offset={}",
                        i,
                        r + 1,
                        repeat,
                        len,
                        offset
                    );
                    failed += 1;
                }
            }
        }
        if failed == 0 {
            Ok(())
        } else {
            error!("read_async: {} iteration(s) failed", failed);
            Err(())
        }
    }

    /// Async SPI DMA `write` (TX-only) over every case from `spi_dma.rs` (single + `mul_3`).
    ///
    /// Each entry in [`WRITE_CASES`] is `(len, offset, repeat)`, where `len` is the TX length
    /// (RX is empty). `repeat == 3` reproduces the `mul_3` tests. The destination slice is empty,
    /// so [`test_write_async`] only checks the under/overflow padding of [`DST_BUF`].
    ///
    /// Every case is run to completion: a failed case is logged with its index and parameters and
    /// the harness moves on to the next one. Returns `Ok(())` only if all cases passed.
    #[test]
    #[timeout(60)]
    async fn write_async((mut spi, crc): (SpiDma, Crc<u32>)) -> Result<(), ()> {
        let mut failed: usize = 0;
        for (
            i,
            &SimplexCase {
                len,
                offset,
                repeat,
            },
        ) in WRITE_CASES.iter().enumerate()
        {
            let dst_buf_size = offset + offset;
            for r in 0..repeat {
                let src = &SRC_BUF[..len];
                let dst = dst_buf(dst_buf_size);
                let res = test_write_async(src, dst, 0, offset, &mut spi, &crc).await;
                if res.is_err() {
                    error!(
                        "write_async: case #{} (iter {}/{}) FAILED — len={}, offset={}",
                        i,
                        r + 1,
                        repeat,
                        len,
                        offset
                    );
                    failed += 1;
                }
            }
        }
        if failed == 0 {
            Ok(())
        } else {
            error!("write_async: {} iteration(s) failed", failed);
            Err(())
        }
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
    if ret.is_err() {
        return Err(());
    }

    test_buffers(src, dst_buf, dst_len, dst_offset, crc)
}

/// Test [`SpiDma::transfer_async`] read (RX-only) transfer
///
/// Mirrors the synchronous `test_read` helper in `spi_dma.rs`: an RX-only transaction is an SPI
/// transfer with an empty TX slice. In loopback mode the received bytes are the filler bytes
/// (`spi::TX_FILLER_BYTE`), which [`test_buffers`] verifies.
async fn test_read_async(
    src: &[u8],
    dst_buf: &mut [u8],
    dst_len: usize,
    dst_offset: usize,
    spi: &mut SpiDma,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    assert_eq!(dst_buf.len(), dst_offset + dst_len + dst_offset);
    let dst = &mut dst_buf[dst_offset..dst_offset + dst_len];

    let ret = spi.transfer_async(dst, &[]).await;
    if ret.is_err() {
        return Err(());
    }

    test_buffers(src, dst_buf, dst_len, dst_offset, crc)
}

/// Test [`SpiDma::transfer_async`] write (TX-only) transfer
///
/// Mirrors the synchronous `test_write` helper in `spi_dma.rs`: a TX-only transaction is an SPI
/// transfer with an empty RX slice. `dst_len` is `0` for write tests, so `dst` is empty and only
/// the under/overflow padding in `dst_buf` is checked by [`test_buffers`].
async fn test_write_async(
    src: &[u8],
    dst_buf: &mut [u8],
    dst_len: usize,
    dst_offset: usize,
    spi: &mut SpiDma,
    crc: &Crc<u32>,
) -> Result<(), ()> {
    let ret = spi.transfer_async(&mut [], src).await;
    if ret.is_err() {
        return Err(());
    }

    test_buffers(src, dst_buf, dst_len, dst_offset, crc)
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
            return Err(());
        }
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
                return Err(());
            }
        }
    }

    // no bytes written before start of `dst`
    for b in &dst_buf[0..dst_offset] {
        if *b != 0 {
            error!(
                "Dst underflow: expected [0;_], found {=[?]}",
                &dst_buf[0..dst_offset]
            );
            return Err(());
        }
    }
    // no bytes written after end of `dst`
    for b in &dst_buf[dst_offset + dst_len..] {
        if *b != 0 {
            error!(
                "Dst overflow: expected [0;_], found {=[?]}",
                &dst_buf[dst_offset + dst_len..]
            );
            return Err(());
        }
    }

    Ok(())
}
