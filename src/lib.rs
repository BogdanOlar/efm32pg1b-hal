//! efm32pg1b-hal
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
//!
#![no_std]
#![warn(missing_docs)]
#![warn(clippy::missing_safety_doc)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// #![warn(clippy::undocumented_unsafe_blocks)]

pub use efm32pg1b_pac as pac;

pub mod cmu;
pub mod crc;
pub mod dma;
pub mod gpio;
pub mod timer;
pub mod timer_le;
pub mod usart;

mod sealed {
    /// Sealed (typestate) marker trait for singleton types.
    /// Used to ensure that certain types may not be instantiated outside this crate.
    pub trait Sealed {}
}

pub(crate) use sealed::Sealed;

/// Convenience module which exports the most used types for each module
pub mod prelude {
    pub use crate::{
        cmu::{CmuExt, HfClockPrescaler, HfClockSource, LfClockSource},
        gpio::{
            pin::mode::{
                Analog, Disabled, DisabledPu, InFilt, InFloat, InPd, InPdFilt, InPu, InPuFilt,
                OutOd, OutOdAlt, OutOdFilt, OutOdFiltAlt, OutOdPu, OutOdPuAlt, OutOdPuFilt,
                OutOdPuFiltAlt, OutOs, OutOsPd, OutPp, OutPpAlt,
            },
            port::{DataInCtrl, DriveStrength},
            Gpio, GpioError,
        },
        usart::spi::{BitOrder, Config, Spi, SpiError, SpiPins},
    };
    pub use efm32pg1b_pac as pac;
    pub use embedded_hal::{
        delay::DelayNs,
        digital::{InputPin, OutputPin, PinState, StatefulOutputPin},
        pwm::SetDutyCycle,
        spi::{self, SpiBus},
    };
}

/// Peripheral single-cycle read-modify-write
///
/// The EFM32 Gecko supports bit set and bit clear access to all peripherals except those listed in
/// Table 4.1 Peripherals that Do Not Support Bit Set and Bit Clear on page 38. The bit set and bit clear functionality
/// (also called Bit Access) enables modification of bit fields (single bit or multiple bit wide) without the need to
/// perform a read-modify-write (though it is functionally equivalent). Also, the operation is contained within a single
/// bus access (for HF peripherals), unlike the Bit-banding operation described in section 4.2.2 Bit-banding which
/// consumes two bus accesses per operation. All AHB masters can utilize this feature.
///
/// See [Documentation](../../doc/efm32pg1-rm.pdf#page919)
trait SingleCycleRMW {
    const BIT_CLEAR_BASE_ADDR: usize = 0x44000000;
    const BIT_SET_BASE_ADDR: usize = 0x46000000;
    const PERIPHERALS_BASE_ADDR: usize = 0x40000000;

    /// Single cycle bit(s) set
    ///
    /// **WARNING**: don't use this for **EMU**, **RMU**, and **CRYOTIMER** peripheral registers!
    fn sc_set(&self, mask: u32);

    /// Single cycle bit(s) clear
    ///
    /// **WARNING**: don't use this for **EMU**, **RMU**, and **CRYOTIMER** peripheral registers!
    fn sc_clear(&self, mask: u32);
}

impl<R> SingleCycleRMW for crate::pac::generic::Reg<R>
where
    R: crate::pac::generic::RegisterSpec,
{
    fn sc_set(&self, mask: u32) {
        let addr = Self::BIT_SET_BASE_ADDR + (self.as_ptr().addr() - Self::PERIPHERALS_BASE_ADDR);
        unsafe { (addr as *mut u32).write_volatile(mask) };
    }

    fn sc_clear(&self, mask: u32) {
        let addr = Self::BIT_CLEAR_BASE_ADDR + (self.as_ptr().addr() - Self::PERIPHERALS_BASE_ADDR);
        unsafe { (addr as *mut u32).write_volatile(mask) };
    }
}
