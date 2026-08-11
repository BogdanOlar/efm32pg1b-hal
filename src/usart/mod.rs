//! Universal Synchronous Asynchronous Receiver/Transmitter
//!
//! This module provides SPI drivers for the USART peripherals

pub mod spi;

/// Helper module for accessing USART register blocks
pub(crate) mod mmio {
    use crate::pac::{usart0::RegisterBlock, Usart0, Usart1};

    /// Get a reference to the `RegisterBlock` of either `Usart0` or `Usart1`
    pub(crate) const fn usartx<const N: u8>() -> &'static RegisterBlock {
        match N {
            0 => unsafe { &*Usart0::ptr() },
            1 => unsafe { &*Usart1::ptr() },
            _ => unreachable!(),
        }
    }

    /// Enable the clock for a USART peripheral
    pub(crate) fn cmu_usart_enable<const N: u8>() {
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfperclken0().modify(|_, w| match N {
            0 => w.usart0().set_bit(),
            1 => w.usart1().set_bit(),
            _ => unreachable!(),
        });
    }

    /// Reset a USART peripheral's registers
    pub(crate) fn reset<const N: u8>() {
        let usart_p = usartx::<N>();

        // Write disable commands first
        usart_p.cmd().write(|w| {
            w.rxdis().set_bit();
            w.txdis().set_bit();
            w.masterdis().set_bit();
            w.rxblockdis().set_bit();
            w.txtridis().set_bit();
            w.cleartx().set_bit();
            w.clearrx().set_bit()
        });

        usart_p.clkdiv().reset();
        usart_p.cmd().reset();
        usart_p.ctrl().reset();
        usart_p.ctrlx().reset();
        usart_p.frame().reset();
        usart_p.i2sctrl().reset();
        usart_p.ien().reset();
        usart_p.ifc().reset();
        usart_p.ifs().reset();
        usart_p.input().reset();
        usart_p.irctrl().reset();
        usart_p.routeloc0().reset();
        usart_p.routeloc1().reset();
        usart_p.routepen().reset();
        usart_p.timecmp0().reset();
        usart_p.timecmp1().reset();
        usart_p.timecmp2().reset();
        usart_p.timing().reset();
        usart_p.trigctrl().reset();
        usart_p.txdata().reset();
        usart_p.txdatax().reset();
        usart_p.txdouble().reset();
        usart_p.txdoublex().reset();
    }
}

/// Marker trait to link a USART peripheral type to its const N index
pub trait UsartIndex<const N: u8> {}

impl UsartIndex<0> for crate::pac::Usart0 {}

impl UsartIndex<1> for crate::pac::Usart1 {}
