//! USART
//!
//! Configure USART peripherals to specific functions (SPI, UART, etc)

use core::fmt;

use crate::{
    pac::Cmu,
    usart::spi::{Spi, UsartClkPin, UsartRxPin, UsartTxPin},
    Sealed,
};
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::Mode,
};

pub mod spi;

pub struct Usart {
    pub usart0: UsartInstance<0>,
    pub usart1: UsartInstance<1>,
}

impl Usart {
    pub fn new(usart0: crate::pac::Usart0, usart1: crate::pac::Usart1) -> Self {
        let mut usart = Self {
            usart0: UsartInstance::<0>::new(usart0),
            usart1: UsartInstance::<1>::new(usart1),
        };

        usart.disable();
        usart.reset();
        usart.enable();

        usart
    }

    fn enable(&mut self) {
        let cmu = unsafe { Cmu::steal() };

        // Enable USART 0 and 1 peripheral clock
        cmu.hfperclken0()
            .modify(|_, w| w.usart0().set_bit().usart1().set_bit());
    }

    fn reset(&mut self) {
        self.reset_instance::<0>();
        self.reset_instance::<1>();
    }

    fn reset_instance<const N: u8>(&mut self) {
        let usart = usarts::usartx::<N>();

        // Write disable commands first
        usart.cmd().write(|w| {
            w.rxdis().set_bit();
            w.txdis().set_bit();
            w.masterdis().set_bit();
            w.rxblockdis().set_bit();
            w.txtridis().set_bit();
            w.cleartx().set_bit();
            w.clearrx().set_bit()
        });

        usart.clkdiv().reset();
        usart.cmd().reset();
        usart.ctrl().reset();
        usart.ctrlx().reset();
        usart.frame().reset();
        usart.i2sctrl().reset();
        usart.ien().reset();
        usart.ifc().reset();
        usart.ifs().reset();
        usart.input().reset();
        usart.irctrl().reset();
        usart.routeloc0().reset();
        usart.routeloc1().reset();
        usart.routepen().reset();
        usart.timecmp0().reset();
        usart.timecmp1().reset();
        usart.timecmp2().reset();
        usart.timing().reset();
        usart.trigctrl().reset();
        usart.txdata().reset();
        usart.txdatax().reset();
        usart.txdouble().reset();
        usart.txdoublex().reset();
    }

    fn disable(&mut self) {
        let cmu = unsafe { Cmu::steal() };

        // Disable USART 0 and 1 peripheral clock
        cmu.hfperclken0()
            .modify(|_, w| w.usart0().clear_bit().usart1().clear_bit());
    }
}

/// Wrapper for each USART PAC peripheral
pub struct UsartInstance<const N: u8> {
    /// FIXME: find some other way to ensure this can't be instantiated from outside the HAL
    pub(crate) _p: (),
}
impl Sealed for UsartInstance<0> {}
impl Sealed for UsartInstance<1> {}

impl<const N: u8> UsartInstance<N>
where
    UsartInstance<N>: Sealed,
{
    pub fn into_spi_bus<PCLK, PTX, PRX>(
        self,
        pin_clk: PCLK,
        pin_tx: PTX,
        pin_rx: PRX,
        mode: Mode,
    ) -> Spi<N, PCLK, PTX, PRX>
    where
        PCLK: OutputPin + UsartClkPin,
        PTX: OutputPin + UsartTxPin,
        PRX: InputPin + UsartRxPin,
    {
        Spi::new(pin_clk, pin_tx, pin_rx, mode)
    }
}

impl UsartInstance<0> {
    pub(crate) fn new(_usart_p: crate::pac::Usart0) -> Self {
        Self { _p: () }
    }

    /// Release the USART0 peripheral used to create this instance
    pub fn free(self) -> crate::pac::Usart0 {
        // [SAFETY]: this struct took ownership of the PAC peripheral when it was created, and no other UsartInstance<0>
        // can be created
        unsafe { crate::pac::Usart0::steal() }
    }
}

impl UsartInstance<1> {
    pub(crate) fn new(_usart_p: crate::pac::Usart1) -> Self {
        Self { _p: () }
    }

    /// Release the USART1 peripheral used to create this instance
    pub fn free(self) -> crate::pac::Usart1 {
        // [SAFETY]: this struct took ownership of the PAC peripheral when it was created, and no other UsartInstance<1>
        // can be created
        unsafe { crate::pac::Usart1::steal() }
    }
}

impl<const N: u8> fmt::Debug for UsartInstance<N>
where
    UsartInstance<N>: Sealed,
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("UsartInstance<{N}>"))
    }
}

#[cfg(feature = "defmt")]
impl<const N: u8> defmt::Format for UsartInstance<N>
where
    UsartInstance<N>: Sealed,
{
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "UsartInstance<{}>", N);
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
