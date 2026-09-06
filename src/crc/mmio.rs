//! Memory mapped IO functions for the CRC peripheral
//!

use crate::{crc::Algorithm, pac::gpcrc::Gpcrc};

pub(crate) fn enable() {
    crc().ctrl().modify(|w| w.set_en(true));
}

pub(crate) fn disable() {
    crc().ctrl().modify(|w| w.set_en(false));
}

pub(crate) fn reset() {
    crc().cmd().write_value(Default::default());
    crc().init().write_value(0);
    crc().poly().write_value(Default::default());
}

pub(crate) fn init() {
    crc().cmd().write(|w| w.set_init(true));
}

pub(crate) fn set_algo_16(algo: &Algorithm<u16>) {
    crc().ctrl().modify(|w| {
        w.set_bitreverse(!algo.refin);
        w.set_polysel(true)
    });
    crc()
        .poly()
        .write(|w| w.set_poly(algo.poly.reverse_bits()));
    crc().init().write_value(algo.init.reverse_bits() as u32);
}

pub(crate) fn set_algo_32(algo: &Algorithm<u32>) {
    crc().ctrl().modify(|w| {
        w.set_bitreverse(!algo.refin);
        w.set_polysel(false)
    });
    crc().init().write_value(algo.init.reverse_bits());
}

pub(crate) fn auto_init_set() {
    crc().ctrl().modify(|w| w.set_autoinit(true));
}

pub(crate) fn input_u8(b: u8) {
    crc()
        .inputdatabyte()
        .write(|w| w.set_inputdatabyte(b));
}

pub(crate) fn data_u16(rev_bits: bool) -> u16 {
    match rev_bits {
        true => (crc().datarev().read() & 0x0000FFFF) as u16,
        false => (crc().data().read() & 0x0000FFFF) as u16,
    }
}

pub(crate) fn data_u32(rev_bits: bool) -> u32 {
    match rev_bits {
        true => crc().datarev().read(),
        false => crc().data().read(),
    }
}

/// Get the CRC (pac) peripheral
fn crc() -> Gpcrc {
    crate::pac::GPCRC
}
