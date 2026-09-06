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
    USART0 = 0,
    /// USART1.
    USART1 = 1,
}

/// Helper module for accessing USART register blocks
pub(crate) mod mmio {
    use crate::pac::{usart0::Timer, USART0, USART1};
    use crate::usart::UsartId;

    /// Get a reference to the `Timer` of either `USART0` or `USART1`
    ///
    /// `id` selects which USART peripheral, as returned by [`UsartIndex::index`](super::UsartIndex::index).
    pub(crate) const fn usartx(id: UsartId) -> &'static Timer {
        match id {
            UsartId::USART0 => USART0,
            UsartId::USART1 => USART1,
        }
    }

    /// Enable the clock for a USART peripheral
    ///
    /// `id` selects which USART peripheral, as returned by [`UsartIndex::index`](super::UsartIndex::index).
    pub(crate) fn cmu_usart_enable(id: UsartId) {
        let cmu = unsafe { crate::pac::CMU::steal() };
        cmu.hfperclken0().modify(|_, w| match id {
            UsartId::USART0 => w.set_usart0(true),
            UsartId::USART1 => w.set_usart1(true),
        });
    }

    /// Reset a USART peripheral's registers
    ///
    /// `id` selects which USART peripheral, as returned by [`UsartIndex::index`](super::UsartIndex::index).
    pub(crate) fn reset(id: UsartId) {
        let usart_p = usartx(id);

        // Write disable commands first
        usart_p.cmd().write(|w| {
            w.set_rxdis(true);
            w.set_txdis(true);
            w.set_masterdis(true);
            w.set_rxblockdis(true);
            w.set_txtridis(true);
            w.set_cleartx(true);
            w.set_clearrx(true)
        });

        usart_p.clkdiv().write_value(Default::default());
        usart_p.cmd().write_value(Default::default());
        usart_p.ctrl().write_value(Default::default());
        usart_p.ctrlx().write_value(Default::default());
        usart_p.frame().write_value(Default::default());
        usart_p.i2sctrl().write_value(Default::default());
        usart_p.ien().write_value(Default::default());
        usart_p.ifc().write_value(Default::default());
        usart_p.ifs().write_value(Default::default());
        usart_p.input().write_value(Default::default());
        usart_p.irctrl().write_value(Default::default());
        usart_p.routeloc0().write_value(Default::default());
        usart_p.routeloc1().write_value(Default::default());
        usart_p.routepen().write_value(Default::default());
        usart_p.timecmp0().write_value(Default::default());
        usart_p.timecmp1().write_value(Default::default());
        usart_p.timecmp2().write_value(Default::default());
        usart_p.timing().write_value(Default::default());
        usart_p.trigctrl().write_value(Default::default());
        usart_p.txdata().write_value(Default::default());
        usart_p.txdatax().write_value(Default::default());
        usart_p.txdouble().write_value(Default::default());
        usart_p.txdoublex().write_value(Default::default());
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

impl UsartIndex for crate::pac::USART0 {
    fn index() -> UsartId {
        UsartId::USART0
    }
}

impl UsartIndex for crate::pac::USART1 {
    fn index() -> UsartId {
        UsartId::USART1
    }
}
