//! Memory mapped IO functions for the CRC peripheral
//!

use crate::{crc::Algorithm, pac::Gpcrc};

pub(crate) fn enable() {
    crc().ctrl().modify(|_, w| w.en().set_bit());
}

pub(crate) fn disable() {
    crc().ctrl().modify(|_, w| w.en().clear_bit());
}

pub(crate) fn reset() {
    crc().cmd().reset();
    crc().init().reset();
    crc().poly().reset();
}

pub(crate) fn init() {
    crc().cmd().write(|w| w.init().set_bit());
}

pub(crate) fn set_algo_16(algo: &Algorithm<u16>) {
    crc().ctrl().modify(|_, w| {
        w.bitreverse().variant(!algo.refin);
        w.polysel().variant(true)
    });
    crc()
        .poly()
        .write(|w| unsafe { w.poly().bits(algo.poly.reverse_bits()) });
    crc()
        .init()
        .write(|w| unsafe { w.init().bits(algo.init.reverse_bits() as u32) });
}

pub(crate) fn set_algo_32(algo: &Algorithm<u32>) {
    crc().ctrl().modify(|_, w| {
        w.bitreverse().variant(!algo.refin);
        w.polysel().variant(false)
    });
    crc()
        .init()
        .write(|w| unsafe { w.init().bits(algo.init.reverse_bits()) });
}

pub(crate) fn auto_init_set() {
    crc().ctrl().modify(|_, w| w.autoinit().set_bit());
}

pub(crate) fn input_u8(b: u8) {
    crc()
        .inputdatabyte()
        .write(|w| unsafe { w.inputdatabyte().bits(b) });
}

pub(crate) fn data_u16(rev_bits: bool) -> u16 {
    match rev_bits {
        true => (crc().datarev().read().datarev().bits() & 0x0000FFFF) as u16,
        false => (crc().data().read().data().bits() & 0x0000FFFF) as u16,
    }
}

pub(crate) fn data_u32(rev_bits: bool) -> u32 {
    match rev_bits {
        true => crc().datarev().read().datarev().bits(),
        false => crc().data().read().data().bits(),
    }
}

/// Get the CRC (pac) peripheral
fn crc() -> Gpcrc {
    unsafe { crate::pac::Gpcrc::steal() }
}
