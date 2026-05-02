//! GPCRC - General Purpose Cyclic Redundancy Check
//!
//! FIXME: expand on this (how crc-rs was used, testing against it, etc)
//! [Source](https://github.com/mrhooray/crc-rs)
//! [Source](https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.legend)

use crate::pac::Gpcrc;

/// Cyclic Redundancy Check driver
#[derive(Debug)]
pub struct Crc {
    _p: Gpcrc,
}

impl Crc {
    /// Create the CRC driver
    pub fn new(p: Gpcrc) -> Self {
        // Enable CRC clock
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfbusclken0().modify(|_, w| w.gpcrc().set_bit());

        Self { _p: p }
    }

    pub fn into_algo_16(self, algo: &Algorithm) -> CrcAlgo16 {
        mmio::reset();
        mmio::set_algo(algo);
        mmio::auto_init_set();
        mmio::enable();
        mmio::init();
        CrcAlgo16 {
            driver: self,
            algo: *algo,
        }
    }
}

#[derive(Debug)]
// #[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CrcAlgo16 {
    driver: Crc,
    algo: Algorithm,
}

impl CrcAlgo16 {
    pub fn update(&self, arr: &[u8]) {
        for b in arr {
            mmio::input_u8(*b);
        }
    }

    pub fn finalize(&self) -> u16 {
        match self.algo.poly {
            CrcPoly::Crc32BZip2 { init: _, xorout: _ } => unreachable!(),
            CrcPoly::Crc16 {
                poly: _,
                init: _,
                xorout,
            } => mmio::data_u16() ^ xorout,
        }
    }

    pub fn release(self) -> Crc {
        self.driver
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Algorithm {
    poly: CrcPoly,
    byte_reverse: bool,
    bit_reverse: bool,
    byte_mode: bool,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CrcPoly {
    Crc32BZip2 { init: u32, xorout: u32 },
    Crc16 { poly: u16, init: u16, xorout: u16 },
}

// /// # [`CRC-16/IBM-SDLC`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xffff`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0xffff`
// /// - `check`: `0x906e`
// /// - `residue`: `0xf0b8`

pub const CRC_16_IBM_SDLC: Algorithm = Algorithm {
    poly: CrcPoly::Crc16 {
        poly: 0x1021,
        init: 0xffff,
        xorout: 0xffff,
    },
    byte_reverse: false,
    bit_reverse: false,
    byte_mode: false,
};

mod mmio {
    use crate::{
        crc::{Algorithm, CrcPoly},
        pac::Gpcrc,
    };

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

    pub(crate) fn set_algo(algo: &Algorithm) {
        crc().ctrl().modify(|_, w| {
            w.bitreverse().variant(algo.bit_reverse);
            w.bytereverse().variant(algo.byte_reverse);
            w.bytemode().variant(algo.byte_mode)
        });

        match algo.poly {
            CrcPoly::Crc32BZip2 { init, xorout: _ } => {
                crc().ctrl().modify(|_, w| w.polysel().variant(false));
                crc().init().write(|w| unsafe { w.init().bits(init) });
            }
            CrcPoly::Crc16 {
                poly,
                init,
                xorout: _,
            } => {
                crc().ctrl().modify(|_, w| w.polysel().variant(true));
                crc()
                    .poly()
                    .write(|w| unsafe { w.poly().bits(poly.reverse_bits()) });
                crc()
                    .init()
                    .write(|w| unsafe { w.init().bits(init as u32) });
            }
        }
    }

    pub(crate) fn input_u8(b: u8) {
        crc()
            .inputdatabyte()
            .write(|w| unsafe { w.inputdatabyte().bits(b) });
    }

    pub(crate) fn data_u16() -> u16 {
        (crc().data().read().data().bits() & 0x0000FFFF) as u16
    }

    /// Get the CRC (pac) peripheral
    fn crc() -> Gpcrc {
        unsafe { crate::pac::Gpcrc::steal() }
    }

    pub(crate) fn auto_init_set() {
        crc().ctrl().modify(|_, w| w.autoinit().set_bit());
    }
}

// #[derive(Debug, Clone, Copy)]
// #[cfg_attr(feature = "defmt", derive(defmt::Format))]
// pub struct Algorithm<W> {
//     poly: W,
//     init: u32,
//     refin: bool,
//     refout: bool,
//     xorout: u32,
//     check: u32,
//     residue: u32,
// }

// /// # [`CRC-32/BZIP2`][1]
// ///
// /// - `width`: `32` bits
// /// - `poly`: `0x4c11db7` (reversed: `0xedb88320`)
// /// - `init`: `0xffffffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0xffffffff`
// /// - `check`: `0xfc891918`
// /// - `residue`: `0xc704dd7b`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-bzip2
// pub const CRC_32_BZIP2: Algorithm<u32> = Algorithm {
//     poly: 0x4c11db7,
//     init: 0xffffffff,
//     refin: false,
//     refout: false,
//     xorout: 0xffffffff,
//     check: 0xfc891918,
//     residue: 0xc704dd7b,
// };

// /// # [`CRC-16/ARC`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0x0`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0xbb3d`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-arc
// pub const CRC_16_ARC: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0x0,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0xbb3d,
//     residue: 0x0,
// };

// /// # [`CRC-16/CDMA2000`][1]
// ///
// /// - `poly`: `0xc867` (reversed: `0xe613`)
// /// - `init`: `0xffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x4c06`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-cdma2000
// pub const CRC_16_CDMA2000: Algorithm<u16> = Algorithm {
//     poly: 0xc867,
//     init: 0xffff,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x4c06,
//     residue: 0x0,
// };

// /// # [`CRC-16/CMS`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0xffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0xaee7`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-cms
// pub const CRC_16_CMS: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0xffff,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0xaee7,
//     residue: 0x0,
// };

// /// # [`CRC-16/DDS-110`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0x800d`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x9ecf`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dds-110
// pub const CRC_16_DDS_110: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0x800d,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x9ecf,
//     residue: 0x0,
// };

// /// # [`CRC-16/DECT-R`][1]
// ///
// /// - `poly`: `0x589` (reversed: `0x91a0`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x1`
// /// - `check`: `0x7e`
// /// - `residue`: `0x589`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dect-r
// pub const CRC_16_DECT_R: Algorithm<u16> = Algorithm {
//     poly: 0x589,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x1,
//     check: 0x7e,
//     residue: 0x589,
// };

// /// # [`CRC-16/DECT-X`][1]
// ///
// /// - `poly`: `0x589` (reversed: `0x91a0`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x7f`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dect-x
// pub const CRC_16_DECT_X: Algorithm<u16> = Algorithm {
//     poly: 0x589,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x7f,
//     residue: 0x0,
// };

// /// # [`CRC-16/DNP`][1]
// ///
// /// - `poly`: `0x3d65` (reversed: `0xa6bc`)
// /// - `init`: `0x0`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0xffff`
// /// - `check`: `0xea82`
// /// - `residue`: `0x66c5`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dnp
// pub const CRC_16_DNP: Algorithm<u16> = Algorithm {
//     poly: 0x3d65,
//     init: 0x0,
//     refin: true,
//     refout: true,
//     xorout: 0xffff,
//     check: 0xea82,
//     residue: 0x66c5,
// };

// /// # [`CRC-16/EN-13757`][1]
// ///
// /// - `poly`: `0x3d65` (reversed: `0xa6bc`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0xffff`
// /// - `check`: `0xc2b7`
// /// - `residue`: `0xa366`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-en-13757
// pub const CRC_16_EN_13757: Algorithm<u16> = Algorithm {
//     poly: 0x3d65,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0xffff,
//     check: 0xc2b7,
//     residue: 0xa366,
// };

// /// # [`CRC-16/GENIBUS`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0xffff`
// /// - `check`: `0xd64e`
// /// - `residue`: `0x1d0f`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-genibus
// pub const CRC_16_GENIBUS: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0xffff,
//     refin: false,
//     refout: false,
//     xorout: 0xffff,
//     check: 0xd64e,
//     residue: 0x1d0f,
// };

// /// # [`CRC-16/GSM`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0xffff`
// /// - `check`: `0xce3c`
// /// - `residue`: `0x1d0f`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-gsm
// pub const CRC_16_GSM: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0xffff,
//     check: 0xce3c,
//     residue: 0x1d0f,
// };

// /// # [`CRC-16/IBM-3740`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x29b1`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-ibm-3740
// pub const CRC_16_IBM_3740: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0xffff,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x29b1,
//     residue: 0x0,
// };

// /// # [`CRC-16/IBM-SDLC`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xffff`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0xffff`
// /// - `check`: `0x906e`
// /// - `residue`: `0xf0b8`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-ibm-sdlc
// pub const CRC_16_IBM_SDLC: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0xffff,
//     refin: true,
//     refout: true,
//     xorout: 0xffff,
//     check: 0x906e,
//     residue: 0xf0b8,
// };

// /// # [`CRC-16/ISO-IEC-14443-3-A`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xc6c6`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0xbf05`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-iso-iec-14443-3-a
// pub const CRC_16_ISO_IEC_14443_3_A: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0xc6c6,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0xbf05,
//     residue: 0x0,
// };

// /// # [`CRC-16/KERMIT`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0x0`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0x2189`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-kermit
// pub const CRC_16_KERMIT: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0x0,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0x2189,
//     residue: 0x0,
// };

// /// # [`CRC-16/LJ1200`][1]
// ///
// /// - `poly`: `0x6f63` (reversed: `0xc6f6`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0xbdf4`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-lj1200
// pub const CRC_16_LJ1200: Algorithm<u16> = Algorithm {
//     poly: 0x6f63,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0xbdf4,
//     residue: 0x0,
// };

// /// # [`CRC-16/M17`][1]
// ///
// /// - `poly`: `0x5935` (reversed: `0xac9a`)
// /// - `init`: `0xffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x772b`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-m17
// pub const CRC_16_M17: Algorithm<u16> = Algorithm {
//     poly: 0x5935,
//     init: 0xffff,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x772b,
//     residue: 0x0,
// };

// /// # [`CRC-16/MAXIM-DOW`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0x0`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0xffff`
// /// - `check`: `0x44c2`
// /// - `residue`: `0xb001`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-maxim-dow
// pub const CRC_16_MAXIM_DOW: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0x0,
//     refin: true,
//     refout: true,
//     xorout: 0xffff,
//     check: 0x44c2,
//     residue: 0xb001,
// };

// /// # [`CRC-16/MCRF4XX`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xffff`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0x6f91`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-mcrf4xx
// pub const CRC_16_MCRF4XX: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0xffff,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0x6f91,
//     residue: 0x0,
// };

// /// # [`CRC-16/MODBUS`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0xffff`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0x4b37`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-modbus
// pub const CRC_16_MODBUS: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0xffff,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0x4b37,
//     residue: 0x0,
// };

// /// # [`CRC-16/NRSC-5`][1]
// ///
// /// - `poly`: `0x80b` (reversed: `0xd010`)
// /// - `init`: `0xffff`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0xa066`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-nrsc-5
// pub const CRC_16_NRSC_5: Algorithm<u16> = Algorithm {
//     poly: 0x80b,
//     init: 0xffff,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0xa066,
//     residue: 0x0,
// };

// /// # [`CRC-16/OPENSAFETY-A`][1]
// ///
// /// - `poly`: `0x5935` (reversed: `0xac9a`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x5d38`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-opensafety-a
// pub const CRC_16_OPENSAFETY_A: Algorithm<u16> = Algorithm {
//     poly: 0x5935,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x5d38,
//     residue: 0x0,
// };

// /// # [`CRC-16/OPENSAFETY-B`][1]
// ///
// /// - `poly`: `0x755b` (reversed: `0xdaae`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x20fe`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-opensafety-b
// pub const CRC_16_OPENSAFETY_B: Algorithm<u16> = Algorithm {
//     poly: 0x755b,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x20fe,
//     residue: 0x0,
// };

// /// # [`CRC-16/PROFIBUS`][1]
// ///
// /// - `poly`: `0x1dcf` (reversed: `0xf3b8`)
// /// - `init`: `0xffff`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0xffff`
// /// - `check`: `0xa819`
// /// - `residue`: `0xe394`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-profibus
// pub const CRC_16_PROFIBUS: Algorithm<u16> = Algorithm {
//     poly: 0x1dcf,
//     init: 0xffff,
//     refin: false,
//     refout: false,
//     xorout: 0xffff,
//     check: 0xa819,
//     residue: 0xe394,
// };

// /// # [`CRC-16/RIELLO`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0xb2aa`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0x63d0`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-riello
// pub const CRC_16_RIELLO: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0xb2aa,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0x63d0,
//     residue: 0x0,
// };

// /// # [`CRC-16/SPI-FUJITSU`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0x1d0f`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0xe5cc`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-spi-fujitsu
// pub const CRC_16_SPI_FUJITSU: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0x1d0f,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0xe5cc,
//     residue: 0x0,
// };

// /// # [`CRC-16/T10-DIF`][1]
// ///
// /// - `poly`: `0x8bb7` (reversed: `0xedd1`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0xd0db`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-t10-dif
// pub const CRC_16_T10_DIF: Algorithm<u16> = Algorithm {
//     poly: 0x8bb7,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0xd0db,
//     residue: 0x0,
// };

// /// # [`CRC-16/TELEDISK`][1]
// ///
// /// - `poly`: `0xa097` (reversed: `0xe905`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0xfb3`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-teledisk
// pub const CRC_16_TELEDISK: Algorithm<u16> = Algorithm {
//     poly: 0xa097,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0xfb3,
//     residue: 0x0,
// };

// /// # [`CRC-16/TMS37157`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0x89ec`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0x0`
// /// - `check`: `0x26b1`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-tms37157
// pub const CRC_16_TMS37157: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0x89ec,
//     refin: true,
//     refout: true,
//     xorout: 0x0,
//     check: 0x26b1,
//     residue: 0x0,
// };

// /// # [`CRC-16/UMTS`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0xfee8`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-umts
// pub const CRC_16_UMTS: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0xfee8,
//     residue: 0x0,
// };

// /// # [`CRC-16/USB`][1]
// ///
// /// - `poly`: `0x8005` (reversed: `0xa001`)
// /// - `init`: `0xffff`
// /// - `refin`: `true`
// /// - `refout`: `true`
// /// - `xorout`: `0xffff`
// /// - `check`: `0xb4c8`
// /// - `residue`: `0xb001`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-usb
// pub const CRC_16_USB: Algorithm<u16> = Algorithm {
//     poly: 0x8005,
//     init: 0xffff,
//     refin: true,
//     refout: true,
//     xorout: 0xffff,
//     check: 0xb4c8,
//     residue: 0xb001,
// };

// /// # [`CRC-16/XMODEM`][1]
// ///
// /// - `poly`: `0x1021` (reversed: `0x8408`)
// /// - `init`: `0x0`
// /// - `refin`: `false`
// /// - `refout`: `false`
// /// - `xorout`: `0x0`
// /// - `check`: `0x31c3`
// /// - `residue`: `0x0`
// ///
// /// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-xmodem
// pub const CRC_16_XMODEM: Algorithm<u16> = Algorithm {
//     poly: 0x1021,
//     init: 0x0,
//     refin: false,
//     refout: false,
//     xorout: 0x0,
//     check: 0x31c3,
//     residue: 0x0,
// };
