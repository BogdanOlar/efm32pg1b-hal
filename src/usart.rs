use core::fmt;

use crate::{cmu::Clocks, gpio::Pin};
use efm32pg1b_pac::{usart0::RegisterBlock, Cmu, Usart0, Usart1};
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::{Error, ErrorKind, ErrorType, SpiBus},
};

use fugit::{HertzU32, RateExtU32};

#[cfg(feature = "defmt")]
use defmt_rtt as _;

/// Extension trait to specialize USART peripheral for a single use (SPI, UART, etc.)
pub trait UsartSpiExt<MCLK, MTX, MRX>
where
    MCLK: OutputPin + UsartClkPin,
    MTX: OutputPin + UsartTxPin,
    MRX: InputPin + UsartRxPin,
{
    type SpiPart;

    /// Configure the USART peripheral as an SPI
    fn into_spi(
        self,
        pin_clk: MCLK,
        pin_tx: MTX,
        pin_rx: MRX,
        baud: HertzU32,
        clocks: &Clocks,
    ) -> Self::SpiPart;
}

impl<MCLK, MTX, MRX> UsartSpiExt<MCLK, MTX, MRX> for Usart1
where
    MCLK: OutputPin + UsartClkPin,
    MTX: OutputPin + UsartTxPin,
    MRX: InputPin + UsartRxPin,
{
    type SpiPart = Spi<1>;

    fn into_spi(
        self,
        pin_clk: MCLK,
        pin_tx: MTX,
        pin_rx: MRX,
        baud: HertzU32,
        clocks: &Clocks,
    ) -> Self::SpiPart {
        // FIXME: Hardcoded USART id <1>
        let usart = usartx::<1>();

        // Enable USART 1 peripheral clock
        unsafe {
            // FIXME: Hardcoded USART1
            Cmu::steal()
                .hfperclken0()
                .modify(|_, w| w.usart1().set_bit());
        };

        // FIXME: Hardcoded USART id <1>
        usartx_reset::<1>();

        usart.ctrl().write(|w| {
            // Set USART to Synchronous Mode
            w.sync().set_bit();
            // Clocl idle low
            w.clkpol().clear_bit();
            // Sample on rising edge
            w.clkpha().clear_bit();
            // Most significant bit first
            w.msbf().set_bit();
            // Disable auto TX
            w.autotx().clear_bit()
        });

        usart.frame().write(|w| {
            // 8 data bits
            w.databits().eight();
            // 1 stop bit
            w.stopbits().one();
            // No parity
            w.parity().none()
        });

        // Set clock divider in order to obtain the closest baudrate to the one requested
        //          USARTn_CLKDIV = 256 x (fHFPERCLK/(2 x brdesired) - 1)
        // We are not bitshifting by `8` because the `div` field starts at bit 3, so we only bitshift by 5
        let clk_div: u32 = ((clocks.hf_per_clk / (baud * 2)) - 1) << 5;
        usart.clkdiv().write(|w| unsafe { w.div().bits(clk_div) });

        // Master enable
        usart.cmd().write(|w| w.masteren().set_bit());

        usart.ctrl().modify(|_, w| {
            // Auto CS
            w.autocs().set_bit();
            // No CS invert
            w.csinv().clear_bit()
        });

        usart.timing().modify(|_, w| {
            w.cshold().zero();
            w.cssetup().zero()
        });

        // Set IO pin routing for Usart
        usart.routeloc0().modify(|_, w| unsafe {
            w.clkloc().bits(pin_clk.loc());
            w.txloc().bits(pin_tx.loc());
            w.rxloc().bits(pin_rx.loc())
        });

        // Enable IO pins for Usart
        usart.routepen().modify(|_, w| {
            w.clkpen().set_bit();
            w.txpen().set_bit();
            w.rxpen().set_bit()
        });

        // Finally, enable UART
        // TODO: if, for eexample, RX would be disabled, then `w.rxddis().set_bit()` should be called instead
        usart.cmd().write(|w| {
            w.rxen().set_bit();
            w.txen().set_bit()
        });

        Self::SpiPart::new()
    }
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Spi<const U: u8> {}

impl<const U: u8> Spi<U> {
    pub fn new() -> Self {
        Spi {}
    }

    pub fn set_loopback(&mut self, enabled: bool) {
        let usart = usartx::<U>();
        usart.ctrl().write(|w| match enabled {
            true => w.loopbk().set_bit(),
            false => w.loopbk().clear_bit(),
        })
    }

    pub fn write(&mut self, buf: &[u8]) {
        let usart = usartx::<U>();
        for b in buf {
            // Wait for TX buffer to be empty (TXBL is set when empty)
            while usart.status().read().txbl().bit_is_clear() {}

            usart.txdata().write(|w| unsafe { w.txdata().bits(*b) });
        }

        // Wait for TX to finish
        while usart.status().read().txc().bit_is_clear() {}
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpiError {
    /// FIXME: add used errors
    Other,
}

impl Error for SpiError {
    fn kind(&self) -> ErrorKind {
        todo!()
    }
}

// Implementations for `Pin` to be used for `embedded-hal` traits
impl<const U: u8> ErrorType for Spi<U> {
    type Error = SpiError;
}

impl<const U: u8> SpiBus for Spi<U> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

const fn usartx<const U: u8>() -> &'static RegisterBlock {
    match U {
        0 => unsafe { &*Usart0::ptr() },
        1 => unsafe { &*Usart1::ptr() },
        _ => unreachable!(),
    }
}

fn usartx_reset<const U: u8>() {
    let usart = usartx::<U>();

    // Use CMD first
    usart.cmd().write(|w| {
        w.rxdis().set_bit();
        w.txdis().set_bit();
        w.masterdis().set_bit();
        w.rxblockdis().set_bit();
        w.txtridis().set_bit();
        w.cleartx().set_bit();
        w.clearrx().set_bit()
    });

    usart.ctrl().reset();
    usart.frame().reset();
    usart.trigctrl().reset();
    usart.clkdiv().reset();
    usart.ien().reset();

    // All flags for the IFC register fields
    const IFC_MASK: u32 = 0x0001FFF9;
    usart.ifc().write(|w| unsafe { w.bits(IFC_MASK) });

    usart.timing().reset();
    usart.routepen().reset();
    usart.routeloc0().reset();
    usart.routeloc1().reset();
    usart.input().reset();

    match U {
        // Only UART0 has IRDA
        0 => usart.irctrl().reset(),
        // Only USART1 has I2S
        1 => usart.i2sctrl().reset(),
        _ => unreachable!(),
    }
}

/// Marker trait to show a pin is can function as a Clock output
pub trait UsartClkPin {
    fn loc(&self) -> u8;
}

/// Marker trait to show a pin is can function as a Tx output
pub trait UsartTxPin {
    fn loc(&self) -> u8;
}

/// Marker trait to show a pin is can function as a Rx input
pub trait UsartRxPin {
    fn loc(&self) -> u8;
}

/// Marker trait to show a pin is can function as a Chip Select output
pub trait UsartCsPin {
    fn loc(&self) -> u8;
}

impl<ANY> UsartClkPin for Pin<'C', 8, ANY> {
    fn loc(&self) -> u8 {
        11
    }
}

impl<ANY> UsartTxPin for Pin<'C', 6, ANY> {
    fn loc(&self) -> u8 {
        11
    }
}

impl<ANY> UsartRxPin for Pin<'C', 7, ANY> {
    fn loc(&self) -> u8 {
        11
    }
}

impl<ANY> UsartCsPin for Pin<'D', 14, ANY> {
    fn loc(&self) -> u8 {
        19
    }
}
