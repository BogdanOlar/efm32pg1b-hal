//! Build with `cargo build --example spi --features="defmt qfn48"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::{
    crc::{algos::CRC_32_CKSUM, CrcDriver},
    dma::{descriptor::Descriptor, Dma},
    prelude::*,
};

use efm32pg1b_pac::Peripherals;
use embedded_hal::spi::MODE_2;
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt::{assert_eq, error, info};
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    // transfer_u8_dma_long_symmetric(Peripherals::take().unwrap());
    transfer_u8_dma_short_asymmetric_rx_longer(Peripherals::take().unwrap());
    loop {}
}

fn transfer_u8_dma_short_asymmetric_rx_longer(p: Peripherals) {
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
    let mut spi = spi.into_spi_dma(dma.ch0, dma.ch1);

    // Set the `dst` length to a multiple of 1
    let mut dst_buf: [u8; SRC_U8_SIZE] = [0; _];

    let src = &SRC_U8[0..16];
    let dst = &mut dst_buf[1..=32];

    let ret_tr1 = spi.transfer(dst, src);
    assert!(ret_tr1.is_ok());
    let ret_tr2 = spi.flush();
    assert!(ret_tr2.is_ok());

    crc.update(src);
    let src_crc = crc.finalize();
    crc.update(&dst[0..src.len()]);
    let dst_crc = crc.finalize();

    // DEBUG:
    if src_crc != dst_crc {
        error!(
            "{} bytes src_crc=0x{:X} dst_crc=0x{:X}",
            src.len().min(dst.len()),
            src_crc,
            dst_crc
        );
        error!("src: {}", src);
        error!("dst: {}", dst);
    }

    assert_eq!(src_crc, dst_crc);

    // check potential under/overflows
    assert_eq!(dst_buf[0], 0);
    assert_eq!(dst_buf[33], 0);
}

/// SPI transfer uses more than one descriptor for both TX and RX
/// TX and RX slices have the same size
fn transfer_u8_dma_long_symmetric(p: Peripherals) {
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
    let mut spi = spi.into_spi_dma(dma.ch1, dma.ch2);

    // Set the `dst` length to a multiple of 1
    let mut dst_buf: [u8; SRC_U8_SIZE] = [0; _];

    let src = &SRC_U8;
    crc.update(src);
    let src_crc = crc.finalize();

    let dst = &mut dst_buf;

    let ret_tr1 = spi.transfer(dst, src);
    assert!(ret_tr1.is_ok());
    let ret_tr2 = spi.flush();
    assert!(ret_tr2.is_ok());

    crc.update(dst);
    let dst_crc = crc.finalize();

    // DEBUG:
    if src_crc != dst_crc {
        error!(
            "src (TX): {} bytes, dst (RX): {} bytes, src_crc=0x{:X} dst_crc=0x{:X}",
            src.len(),
            dst.len(),
            src_crc,
            dst_crc
        );
        error!("src: {}", src[src.len() - 10..src.len()]);
        error!("dst: {}", dst[dst.len() - 10..dst.len()]);
    } else {
        info!(
            "src (TX): {} bytes, dst (RX): {} bytes, src_crc=0x{:X} dst_crc=0x{:X}",
            src.len(),
            dst.len(),
            src_crc,
            dst_crc
        );
    }

    assert_eq!(src_crc, dst_crc);
}

/// `Descriptor::MAX_TRANSFER_UNITS` = `0x800` = `2048` bytes
///
/// The size of RAM is 32K, and since the tests may use a destination (RX) buffer of size `SRC_U8_SIZE`, then
/// the value needs to be smaller than 32K (probably even smaler than that)
const SRC_U8_SIZE: usize = Descriptor::MAX_TRANSFER_UNITS * 14;

#[allow(clippy::large_const_arrays)]
const SRC_U8: [u8; SRC_U8_SIZE] = {
    let mut seq = [0; SRC_U8_SIZE];
    let mut i = 0;
    // Fill the buffer with values from 1 to 254 (`0x00` is reserved for the initial contents of the RX buffer,
    // and `0xff` for the filler value for TX transactions where TX is smaller than RX
    while i < SRC_U8_SIZE {
        seq[i] = 1 + (i % (u8::MAX as usize - 1)) as u8;
        i += 1;
    }
    seq
};
