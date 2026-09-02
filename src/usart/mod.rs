//! Universal Synchronous Asynchronous Receiver/Transmitter
//!
//! This module provides SPI drivers for the USART peripherals

pub mod spi;

/// Identifies which USART peripheral a driver instance is bound to.
///
/// `Spi` is a specialisation of the USART peripheral, so this runtime identifier lives at the
/// `usart` module level: each PAC USART type maps to one [`UsartId`] via [`UsartIndex::index`],
/// and the drivers store a `UsartId` (rather than a raw `u8`) to make the peripheral selection
/// self-documenting and exhaustive at every `match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum UsartId {
    /// USART0.
    Usart0 = 0,
    /// USART1.
    Usart1 = 1,
}

/// Helper module for accessing USART register blocks
pub(crate) mod mmio {
    use crate::pac::{usart0::RegisterBlock, Usart0, Usart1};
    use crate::usart::UsartId;

    /// Get a reference to the `RegisterBlock` of either `Usart0` or `Usart1`
    ///
    /// `id` selects which USART peripheral, as returned by [`UsartIndex::index`](super::UsartIndex::index).
    pub(crate) const fn usartx(id: UsartId) -> &'static RegisterBlock {
        match id {
            UsartId::Usart0 => unsafe { &*Usart0::ptr() },
            UsartId::Usart1 => unsafe { &*Usart1::ptr() },
        }
    }

    /// Enable the clock for a USART peripheral
    ///
    /// `id` selects which USART peripheral, as returned by [`UsartIndex::index`](super::UsartIndex::index).
    pub(crate) fn cmu_usart_enable(id: UsartId) {
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfperclken0().modify(|_, w| match id {
            UsartId::Usart0 => w.usart0().set_bit(),
            UsartId::Usart1 => w.usart1().set_bit(),
        });
    }

    /// Reset a USART peripheral's registers
    ///
    /// `id` selects which USART peripheral, as returned by [`UsartIndex::index`](super::UsartIndex::index).
    pub(crate) fn reset(id: UsartId) {
        let usart_p = usartx(id);

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

/// Marker trait to link a USART peripheral type to its runtime [`UsartId`].
///
/// The [`UsartIndex::index`] associated function returns the [`UsartId`] used to route register
/// accesses at runtime, allowing drivers such as [`spi::Spi`](crate::usart::spi::Spi) to be
/// non-generic over the peripheral.
pub trait UsartIndex {
    /// Runtime [`UsartId`] of this USART peripheral.
    fn index() -> UsartId;
}

impl UsartIndex for crate::pac::Usart0 {
    fn index() -> UsartId {
        UsartId::Usart0
    }
}

impl UsartIndex for crate::pac::Usart1 {
    fn index() -> UsartId {
        UsartId::Usart1
    }
}
