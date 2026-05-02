//! Well known CRC algos this HAL can calculate: CRC-16, and also the CRC-32 with polynomial `0x4c11db7` (`IEEE 802.3`)
//!
//! Converted compatible algorithms from the [`crc`](https://github.com/mrhooray/crc-rs) crate.
//!
//! If the crate feature `crc-lib-compat` is enabled, then a `From` implementation is also available in the `lib_compat`
//! module which can convert `crc::Algorithm<u16>` into `crate::crc::Algorithm<u16>`. The CRC peripheral of this uC can
//! only calculate `Algorithm<u32>` with the `0x4c11db7` polynomial, so all algos from `crc` crate have been converted
//! manually in this module.
//!

use crate::crc::Algorithm;

/// Convert `crc` crate CRC16 algos into this HAL's [`Algorithm<u16>`]
#[cfg(feature = "crc-lib-compat")]
pub mod lib_compat {
    use crc;

    impl From<crc::Algorithm<u16>> for crate::crc::Algorithm<u16> {
        fn from(value: crc::Algorithm<u16>) -> Self {
            Self::new(
                value.poly,
                value.init,
                value.xorout,
                value.refin,
                value.refout,
            )
        }
    }
}

/// # [`CRC-16/ARC`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0x0`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0xbb3d`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-arc
pub const CRC_16_ARC: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0x0, 0x0, true, true);

/// # [`CRC-16/CDMA2000`][1]
///
/// - `poly`: `0xc867` (reversed: `0xe613`)
/// - `init`: `0xffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x4c06`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-cdma2000
pub const CRC_16_CDMA2000: Algorithm<u16> =
    Algorithm::<u16>::new(0xc867, 0xffff, 0x0, false, false);

/// # [`CRC-16/CMS`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0xffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0xaee7`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-cms
pub const CRC_16_CMS: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0xffff, 0x0, false, false);

/// # [`CRC-16/DDS-110`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0x800d`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x9ecf`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dds-110
pub const CRC_16_DDS_110: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0x800d, 0x0, false, false);

/// # [`CRC-16/DECT-R`][1]
///
/// - `poly`: `0x589` (reversed: `0x91a0`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x1`
/// - `check`: `0x7e`
/// - `residue`: `0x589`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dect-r
pub const CRC_16_DECT_R: Algorithm<u16> = Algorithm::<u16>::new(0x589, 0x0, 0x1, false, false);

/// # [`CRC-16/DECT-X`][1]
///
/// - `poly`: `0x589` (reversed: `0x91a0`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x7f`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dect-x
pub const CRC_16_DECT_X: Algorithm<u16> = Algorithm::<u16>::new(0x589, 0x0, 0x0, false, false);

/// # [`CRC-16/DNP`][1]
///
/// - `poly`: `0x3d65` (reversed: `0xa6bc`)
/// - `init`: `0x0`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0xffff`
/// - `check`: `0xea82`
/// - `residue`: `0x66c5`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-dnp
pub const CRC_16_DNP: Algorithm<u16> = Algorithm::<u16>::new(0x3d65, 0x0, 0xffff, true, true);

/// # [`CRC-16/EN-13757`][1]
///
/// - `poly`: `0x3d65` (reversed: `0xa6bc`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0xffff`
/// - `check`: `0xc2b7`
/// - `residue`: `0xa366`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-en-13757
pub const CRC_16_EN_13757: Algorithm<u16> =
    Algorithm::<u16>::new(0x3d65, 0x0, 0xffff, false, false);

/// # [`CRC-16/GENIBUS`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0xffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0xffff`
/// - `check`: `0xd64e`
/// - `residue`: `0x1d0f`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-genibus
pub const CRC_16_GENIBUS: Algorithm<u16> =
    Algorithm::<u16>::new(0x1021, 0xffff, 0xffff, false, false);

/// # [`CRC-16/GSM`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0xffff`
/// - `check`: `0xce3c`
/// - `residue`: `0x1d0f`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-gsm
pub const CRC_16_GSM: Algorithm<u16> = Algorithm::<u16>::new(0x1021, 0x0, 0xffff, false, false);

/// # [`CRC-16/IBM-3740`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0xffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x29b1`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-ibm-3740
pub const CRC_16_IBM_3740: Algorithm<u16> =
    Algorithm::<u16>::new(0x1021, 0xffff, 0x0, false, false);

/// # [`CRC-16/IBM-SDLC`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0xffff`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0xffff`
/// - `check`: `0x906e`
/// - `residue`: `0xf0b8`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-ibm-sdlc
pub const CRC_16_IBM_SDLC: Algorithm<u16> =
    Algorithm::<u16>::new(0x1021, 0xffff, 0xffff, true, true);

/// # [`CRC-16/ISO-IEC-14443-3-A`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0xc6c6`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0xbf05`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-iso-iec-14443-3-a
pub const CRC_16_ISO_IEC_14443_3_A: Algorithm<u16> =
    Algorithm::<u16>::new(0x1021, 0xc6c6, 0x0, true, true);

/// # [`CRC-16/KERMIT`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0x0`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0x2189`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-kermit
pub const CRC_16_KERMIT: Algorithm<u16> = Algorithm::<u16>::new(0x1021, 0x0, 0x0, true, true);

/// # [`CRC-16/LJ1200`][1]
///
/// - `poly`: `0x6f63` (reversed: `0xc6f6`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0xbdf4`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-lj1200
pub const CRC_16_LJ1200: Algorithm<u16> = Algorithm::<u16>::new(0x6f63, 0x0, 0x0, false, false);

/// # [`CRC-16/M17`][1]
///
/// - `poly`: `0x5935` (reversed: `0xac9a`)
/// - `init`: `0xffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x772b`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-m17
pub const CRC_16_M17: Algorithm<u16> = Algorithm::<u16>::new(0x5935, 0xffff, 0x0, false, false);

/// # [`CRC-16/MAXIM-DOW`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0x0`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0xffff`
/// - `check`: `0x44c2`
/// - `residue`: `0xb001`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-maxim-dow
pub const CRC_16_MAXIM_DOW: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0x0, 0xffff, true, true);

/// # [`CRC-16/MCRF4XX`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0xffff`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0x6f91`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-mcrf4xx
pub const CRC_16_MCRF4XX: Algorithm<u16> = Algorithm::<u16>::new(0x1021, 0xffff, 0x0, true, true);

/// # [`CRC-16/MODBUS`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0xffff`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0x4b37`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-modbus
pub const CRC_16_MODBUS: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0xffff, 0x0, true, true);

/// # [`CRC-16/NRSC-5`][1]
///
/// - `poly`: `0x80b` (reversed: `0xd010`)
/// - `init`: `0xffff`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0xa066`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-nrsc-5
pub const CRC_16_NRSC_5: Algorithm<u16> = Algorithm::<u16>::new(0x80b, 0xffff, 0x0, true, true);

/// # [`CRC-16/OPENSAFETY-A`][1]
///
/// - `poly`: `0x5935` (reversed: `0xac9a`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x5d38`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-opensafety-a
pub const CRC_16_OPENSAFETY_A: Algorithm<u16> =
    Algorithm::<u16>::new(0x5935, 0x0, 0x0, false, false);

/// # [`CRC-16/OPENSAFETY-B`][1]
///
/// - `poly`: `0x755b` (reversed: `0xdaae`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x20fe`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-opensafety-b
pub const CRC_16_OPENSAFETY_B: Algorithm<u16> =
    Algorithm::<u16>::new(0x755b, 0x0, 0x0, false, false);

/// # [`CRC-16/PROFIBUS`][1]
///
/// - `poly`: `0x1dcf` (reversed: `0xf3b8`)
/// - `init`: `0xffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0xffff`
/// - `check`: `0xa819`
/// - `residue`: `0xe394`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-profibus
pub const CRC_16_PROFIBUS: Algorithm<u16> =
    Algorithm::<u16>::new(0x1dcf, 0xffff, 0xffff, false, false);

/// # [`CRC-16/RIELLO`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0xb2aa`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0x63d0`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-riello
pub const CRC_16_RIELLO: Algorithm<u16> = Algorithm::<u16>::new(0x1021, 0xb2aa, 0x0, true, true);

/// # [`CRC-16/SPI-FUJITSU`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0x1d0f`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0xe5cc`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-spi-fujitsu
pub const CRC_16_SPI_FUJITSU: Algorithm<u16> =
    Algorithm::<u16>::new(0x1021, 0x1d0f, 0x0, false, false);

/// # [`CRC-16/T10-DIF`][1]
///
/// - `poly`: `0x8bb7` (reversed: `0xedd1`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0xd0db`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-t10-dif
pub const CRC_16_T10_DIF: Algorithm<u16> = Algorithm::<u16>::new(0x8bb7, 0x0, 0x0, false, false);

/// # [`CRC-16/TELEDISK`][1]
///
/// - `poly`: `0xa097` (reversed: `0xe905`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0xfb3`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-teledisk
pub const CRC_16_TELEDISK: Algorithm<u16> = Algorithm::<u16>::new(0xa097, 0x0, 0x0, false, false);

/// # [`CRC-16/TMS37157`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0x89ec`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0x0`
/// - `check`: `0x26b1`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-tms37157
pub const CRC_16_TMS37157: Algorithm<u16> = Algorithm::<u16>::new(0x1021, 0x89ec, 0x0, true, true);

/// # [`CRC-16/UMTS`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0xfee8`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-umts
pub const CRC_16_UMTS: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0x0, 0x0, false, false);

/// # [`CRC-16/USB`][1]
///
/// - `poly`: `0x8005` (reversed: `0xa001`)
/// - `init`: `0xffff`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `xorout`: `0xffff`
/// - `check`: `0xb4c8`
/// - `residue`: `0xb001`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-usb
pub const CRC_16_USB: Algorithm<u16> = Algorithm::<u16>::new(0x8005, 0xffff, 0xffff, true, true);

/// # [`CRC-16/XMODEM`][1]
///
/// - `poly`: `0x1021` (reversed: `0x8408`)
/// - `init`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `xorout`: `0x0`
/// - `check`: `0x31c3`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-xmodem
pub const CRC_16_XMODEM: Algorithm<u16> = Algorithm::<u16>::new(0x1021, 0x0, 0x0, false, false);

/// # [`CRC-32/BZIP2`][1]
///
/// - `poly`: `0x4c11db7` (reversed: `0xedb88320`)
/// - `init`: `0xffffffff`
/// - `xorout`: `0xffffffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `check`: `0xfc891918`
/// - `residue`: `0xc704dd7b`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-bzip2
pub const CRC_32_BZIP2: Algorithm<u32> =
    Algorithm::<u32>::new(0xffffffff, 0xffffffff, false, false);

/// # [`CRC-32/CKSUM`][1]
///
/// - `poly`: `0x4c11db7` (reversed: `0xedb88320`)
/// - `init`: `0x0`
/// - `xorout`: `0xffffffff`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `check`: `0x765e7680`
/// - `residue`: `0xc704dd7b`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-cksum
pub const CRC_32_CKSUM: Algorithm<u32> = Algorithm::<u32>::new(0x0, 0xffffffff, false, false);

/// # [`CRC-32/ISO-HDLC`][1]
///
/// - `poly`: `0x4c11db7` (reversed: `0xedb88320`)
/// - `init`: `0xffffffff`
/// - `xorout`: `0xffffffff`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `check`: `0xcbf43926`
/// - `residue`: `0xdebb20e3`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-iso-hdlc
pub const CRC_32_ISO_HDLC: Algorithm<u32> =
    Algorithm::<u32>::new(0xffffffff, 0xffffffff, true, true);

/// # [`CRC-32/JAMCRC`][1]
///
/// - `poly`: `0x4c11db7` (reversed: `0xedb88320`)
/// - `init`: `0xffffffff`
/// - `xorout`: `0x0`
/// - `refin`: `true`
/// - `refout`: `true`
/// - `check`: `0x340bc6d9`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-jamcrc
pub const CRC_32_JAMCRC: Algorithm<u32> = Algorithm::<u32>::new(0xffffffff, 0x0, true, true);

/// # [`CRC-32/MPEG-2`][1]
///
/// - `poly`: `0x4c11db7` (reversed: `0xedb88320`)
/// - `init`: `0xffffffff`
/// - `xorout`: `0x0`
/// - `refin`: `false`
/// - `refout`: `false`
/// - `check`: `0x376e6e7`
/// - `residue`: `0x0`
///
/// [1]: https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-mpeg-2
pub const CRC_32_MPEG_2: Algorithm<u32> = Algorithm::<u32>::new(0xffffffff, 0x0, false, false);
