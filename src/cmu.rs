//! Clock Managened Unit (CMU) API
use crate::pac::*;

pub trait CmuExt {
    fn freeze(self) -> Cmu;
}

impl CmuExt for CMU {
    // TODO: Make this configurable
    fn freeze(self) -> Cmu {
        Cmu {
            raw: self,
            hfclk: 19_000_000,
        }
    }
}

pub struct Cmu {
    raw: CMU,
    hfclk: u32,
}

impl Cmu {
    /// This clock drives the Core Modules, which consists of the CPU and modules
    /// that are tightly coupled to the CPU, e.g. the cache.
    pub fn hfcoreclk(&self) -> u32 {
        self.hfclk / (self.raw.hfcorepresc.read().bits() + 1)
    }

    /// This clock drives the Bus and Memory System. It is also used to drive the
    /// bus interface to the Low Energy Peripherals.
    pub fn hfbusclk(&self) -> u32 {
        self.hfclk
    }

    /// This clock drives the High-Frequency Peripherals.
    pub fn hfperclk(&self) -> u32 {
        self.hfclk / (self.raw.hfperpresc.read().bits() + 1)
    }

    /// Enables all clocks required to use a peripheral.
    pub fn enable_clock(&mut self, peripheral: &impl ClockControlExt) {
        peripheral.enable_clock(self);
    }
}

pub trait ClockControlExt {
    fn enable_clock(&self, clocks: &mut Cmu);
}

macro_rules! impl_clock_control_ext {
    ($type:ty, $reg:ident, $bit:ident) => {
        impl ClockControlExt for $type {
            fn enable_clock(&self, clocks: &mut Cmu) {
                clocks.raw.$reg.modify(|_, w| w.$bit().set_bit());
            }
        }
    };
}

// impl_clock_control_ext!(CRYPTO0, hfbusclken0, crypto0);
// impl_clock_control_ext!(CRYPTO1, hfbusclken0, crypto1);
impl_clock_control_ext!(GPIO, hfbusclken0, gpio);
impl_clock_control_ext!(PRS, hfbusclken0, prs);
impl_clock_control_ext!(LDMA, hfbusclken0, ldma);
impl_clock_control_ext!(GPCRC, hfbusclken0, gpcrc);

impl_clock_control_ext!(TIMER0, hfperclken0, timer0);
impl_clock_control_ext!(TIMER1, hfperclken0, timer1);
// impl_clock_control_ext!(WTIMER0, hfperclken0, wtimer0);
// impl_clock_control_ext!(WTIMER1, hfperclken0, wtimer1);
impl_clock_control_ext!(USART0, hfperclken0, usart0);
impl_clock_control_ext!(USART1, hfperclken0, usart1);
// impl_clock_control_ext!(USART2, hfperclken0, usart2);
// impl_clock_control_ext!(USART3, hfperclken0, usart3);
impl_clock_control_ext!(I2C0, hfperclken0, i2c0);
// impl_clock_control_ext!(I2C1, hfperclken0, i2c1);
impl_clock_control_ext!(ACMP0, hfperclken0, acmp0);
impl_clock_control_ext!(ACMP1, hfperclken0, acmp1);
impl_clock_control_ext!(CRYOTIMER, hfperclken0, cryotimer);
impl_clock_control_ext!(ADC0, hfperclken0, adc0);
impl_clock_control_ext!(IDAC0, hfperclken0, idac0);
// impl_clock_control_ext!(VDAC0, hfperclken0, vdac0);
// impl_clock_control_ext!(TRNG0, hfperclken0, trng0);

macro_rules! impl_lf_clock_control_ext {
    ($type:ty, $reg:ident, $bit:ident) => {
        impl ClockControlExt for $type {
            fn enable_clock(&self, clocks: &mut Cmu) {
                // To access LF peripheral registers the HFBUSCLKLE clock must be enabled.
                clocks.raw.hfbusclken0.modify(|_, w| w.le().set_bit());
                clocks.raw.$reg.modify(|_, w| w.$bit().set_bit());
            }
        }
    };
}

impl_lf_clock_control_ext!(LETIMER0, lfaclken0, letimer0);
// impl_lf_clock_control_ext!(LESENSE, lfaclken0, lesense);
// impl_lf_clock_control_ext!(SYST, lfbclken0, systick);
impl_lf_clock_control_ext!(LEUART0, lfbclken0, leuart0);
impl_lf_clock_control_ext!(RTCC, lfeclken0, rtcc);

// // The CSEN peripheral is special because it uses the HF and LF clock domain.
// impl ClockControlExt for CSEN {
//     fn enable_clock(&self, clocks: &mut Cmu) {
//         clocks.raw.hfperclken0.modify(|_, w| w.csen().set_bit());
//         clocks.raw.lfbclken0.modify(|_, w| w.csen().set_bit());
//     }
// }
