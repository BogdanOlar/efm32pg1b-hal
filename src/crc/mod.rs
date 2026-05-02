//! GPCRC - General Purpose Cyclic Redundancy Check
//!
//! # Blocking
//!
//! ```rust,no_run
//!     let p = Peripherals::take().unwrap();
//!
//!     let data: &[u8] = "123456789".as_bytes();
//!
//!     let driver: Crc = Crc::new(p.gpcrc);
//!
//!     // Create a CRC with a particular Algorithm
//!     let crc_algo: CrcAlgo<u16> = driver.into_algo_16(&efm32pg1b_hal::crc::algos::CRC_16_ARC);
//!
//!     // calculate CRC in one go
//!     crc_algo.update(data);
//!     let crc = crc_algo.finalize();
//!     assert_eq!(crc, 0xbb3d);
//!
//!     // or calculate CRC in multiple calls (CRC algo was reset to initial state when `finalize()` was called above)
//!     crc_algo.update(&data[..data.len() / 2]);
//!     crc_algo.update(&data[data.len() / 2..]);
//!     let crc = crc_algo.finalize();
//!     assert_eq!(crc, 0xbb3d);
//!
//!     let driver = crc_algo.release();
//!
//!     // use another algo (CRC-32)
//!     let crc_algo: CrcAlgo<u32> = driver.into_algo_32(&efm32pg1b_hal::crc::algos::CRC_32_CKSUM);
//!     for b in data {
//!         crc_algo.update(&[*b]);
//!     }
//!     let crc = crc_algo.finalize();
//!     assert_eq!(crc, 0x765e7680);
//! ```

pub mod algos;
use crate::pac::Gpcrc;

/// Cyclic Redundancy Check driver
#[derive(Debug)]
pub struct Crc {
    p: Gpcrc,
}

impl Crc {
    /// Create the CRC driver
    pub fn new(p: Gpcrc) -> Self {
        // Enable CRC clock
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfbusclken0().modify(|_, w| w.gpcrc().set_bit());

        Self { p }
    }

    /// Create a CRC-16 algo
    pub fn into_algo_16(self, algo: &Algorithm<u16>) -> CrcAlgo<u16> {
        mmio::reset();
        mmio::set_algo_16(algo);
        mmio::auto_init_set();
        mmio::enable();
        mmio::init();
        CrcAlgo {
            driver: self,
            xorout: algo.xorout,
            refout: algo.refout,
        }
    }

    /// Create a CRC-32 algo
    pub fn into_algo_32(self, algo: &Algorithm<u32>) -> CrcAlgo<u32> {
        mmio::reset();
        mmio::set_algo_32(algo);
        mmio::auto_init_set();
        mmio::enable();
        mmio::init();
        CrcAlgo {
            driver: self,
            xorout: algo.xorout,
            refout: algo.refout,
        }
    }

    /// Destroy the CRC driver and release the GPCRC peripheral
    pub fn release(self) -> Gpcrc {
        // Disable CRC clock
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfbusclken0().modify(|_, w| w.gpcrc().clear_bit());

        self.p
    }
}

/// CRC algorithm
#[derive(Debug)]
// #[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CrcAlgo<W> {
    driver: Crc,
    xorout: W,
    refout: bool,
}

impl<W> CrcAlgo<W> {
    /// Push new data
    pub fn update(&self, arr: &[u8]) {
        for b in arr {
            mmio::input_u8(*b);
        }
    }

    /// Destroy this Algo and return the CRC driver used to create it
    pub fn release(self) -> Crc {
        mmio::disable();
        self.driver
    }
}

impl CrcAlgo<u16> {
    /// Finalize the CrcAlgo and return the resulted CRC-16
    ///
    /// After calling this method, the CrcAlgo can be used again to calculate a new CRC with the same [`Algorithm`]
    pub fn finalize(&self) -> u16 {
        mmio::data_u16(!self.refout) ^ self.xorout
    }
}

impl CrcAlgo<u32> {
    /// Finalize the CrcAlgo and return the resulted CRC-32
    ///
    /// After calling this method, the CrcAlgo can be used again to calculate a new CRC with the same [`Algorithm`]
    pub fn finalize(&self) -> u32 {
        mmio::data_u32(!self.refout) ^ self.xorout
    }
}

/// CRC algorithm
///
/// Can be either 16-bit CRC with any polynomial, or a 32-bit CRC with the fixed `0x04C11DB7` polynomial
///
/// [See the crc-catalogue](https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.legend)
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Algorithm<W> {
    poly: W,
    init: W,
    xorout: W,
    refin: bool,
    refout: bool,
}

impl Algorithm<u16> {
    /// Create a CRC-16 algo
    pub const fn new(poly: u16, init: u16, xorout: u16, refin: bool, refout: bool) -> Self {
        Self {
            poly,
            init,
            xorout,
            refin,
            refout,
        }
    }
}

impl Algorithm<u32> {
    /// Create the 32-bit `IEEE 802.3` CRC algo (`0x04C11DB7` polynomial)
    pub const fn new(init: u32, xorout: u32, refin: bool, refout: bool) -> Self {
        Self {
            poly: 0x04C11DB7,
            init,
            xorout,
            refin,
            refout,
        }
    }
}

mod mmio {
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
}
