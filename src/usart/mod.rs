//! USART
//!
//! Usart driver for either [`Usart0`](`crate::pac::Usart0`) or [`Usart1`](`crate::pac::Usart1`) PAC peripherals
//!
//! Is responsible for specialising the Usart into specific functions (SPI, UART, etc)
//!
//! The corresponding clock for USART0 or USART1 is only enabled when the Usart is specialised into Spi, Uart, etc, and
//! is disabled when the Usart is freed with [`Usart::free`](`crate::usart::Usart::free`)

use crate::{
    pac::Cmu,
    usart::{
        spi::{Spi, UsartClkPin, UsartRxPin, UsartTxPin},
        usarts::usartx,
    },
    Sealed,
};
use core::fmt;
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::Mode,
};

pub mod spi;

/// Helper trait to create/free `Usart` instances from either [`Usart0`](`crate::pac::Usart0`) or
/// [`Usart1`](`crate::pac::Usart1`)
pub trait UsartBuild<const N: u8, USART>: Sealed {
    /// Create a Usart driver using one of the PAC peripherals:
    /// [`Usart0`](`crate::pac::Usart0`) or [`Usart1`](`crate::pac::Usart1`)
    fn new(usart_p: USART) -> Self;

    /// Free the PAC peripheral used to create this driver, and disable the corresponding USART peripheral clock
    fn free(self) -> USART;
}

impl UsartBuild<0, crate::pac::Usart0> for Usart<0> {
    fn new(_usart_p: crate::pac::Usart0) -> Self {
        let mut usart = Self { _p: () };
        usart.reset();
        usart
    }

    fn free(mut self) -> crate::pac::Usart0 {
        self.reset();
        self.disable();
        unsafe { crate::pac::Usart0::steal() }
    }
}

impl UsartBuild<1, crate::pac::Usart1> for Usart<1> {
    fn new(_usart_p: crate::pac::Usart1) -> Self {
        let mut usart = Self { _p: () };
        usart.reset();
        usart
    }

    fn free(mut self) -> crate::pac::Usart1 {
        self.reset();
        self.disable();
        unsafe { crate::pac::Usart1::steal() }
    }
}

/// Usart driver
pub struct Usart<const N: u8> {
    _p: (),
}

impl<const N: u8> Usart<N> {
    pub fn into_spi_bus<PCLK, PTX, PRX>(
        mut self,
        pin_clk: PCLK,
        pin_tx: PTX,
        pin_rx: PRX,
        mode: Mode,
    ) -> Spi<N, Usart<N>, PCLK, PTX, PRX>
    where
        PCLK: OutputPin + UsartClkPin,
        PTX: OutputPin + UsartTxPin,
        PRX: InputPin + UsartRxPin,
    {
        self.enable();
        Spi::new(self, pin_clk, pin_tx, pin_rx, mode)
    }

    fn enable(&mut self) {
        let cmu = unsafe { Cmu::steal() };

        cmu.hfperclken0().modify(|_, w| match N {
            0 => w.usart0().set_bit(),
            1 => w.usart1().set_bit(),
            _ => unreachable!(),
        });
    }

    fn disable(&mut self) {
        let cmu = unsafe { Cmu::steal() };

        cmu.hfperclken0().modify(|_, w| match N {
            0 => w.usart0().clear_bit(),
            1 => w.usart1().clear_bit(),
            _ => unreachable!(),
        });
    }

    fn reset(&mut self) {
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

impl Sealed for Usart<0> {}
impl Sealed for Usart<1> {}

impl<const N: u8> fmt::Debug for Usart<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("Usart<{N}>"))
    }
}

#[cfg(feature = "defmt")]
impl<const N: u8> defmt::Format for Usart<N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Usart<{}>", N);
    }
}

pub(crate) mod usarts {
    use crate::pac::{usart0::RegisterBlock, Usart0, Usart1};

    /// Get a reference to the `RegisterBlock` of either `Usart0` or `Usart1`
    pub(crate) const fn usartx<const N: u8>() -> &'static RegisterBlock {
        match N {
            0 => unsafe { &*Usart0::ptr() },
            1 => unsafe { &*Usart1::ptr() },
            _ => unreachable!(),
        }
    }
}
