use crate::{cmu::Clocks, gpio::Pin};
use efm32pg1b_pac::{usart0::RegisterBlock, Cmu, Usart0, Usart1};
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::{Error, ErrorKind, ErrorType, SpiBus},
};
use fugit::{HertzU32, RateExtU32};

#[cfg(feature = "defmt")]
use defmt_rtt as _;

/// Get a reference to the `RegisterBlock` of either `Usart0` or `Usart1`
const fn usartx<const N: u8>() -> &'static RegisterBlock {
    match N {
        0 => unsafe { &*Usart0::ptr() },
        1 => unsafe { &*Usart1::ptr() },
        _ => unreachable!(),
    }
}

/// Extension trait to specialize USART peripheral for SPI
pub trait UsartSpiExt<MCLK, MTX, MRX> {
    type SpiPart;

    /// Configure the USART peripheral as an SPI
    fn into_spi_bus(self, pin_clk: MCLK, pin_tx: MTX, pin_rx: MRX) -> Self::SpiPart;
}

impl<MCLK, MTX, MRX> UsartSpiExt<MCLK, MTX, MRX> for Usart0
where
    MCLK: OutputPin + UsartClkPin,
    MTX: OutputPin + UsartTxPin,
    MRX: InputPin + UsartRxPin,
{
    type SpiPart = Spi<0>;

    fn into_spi_bus(self, pin_clk: MCLK, pin_tx: MTX, pin_rx: MRX) -> Self::SpiPart {
        // Enable USART 1 peripheral clock
        unsafe {
            // FIXME: Hardcoded USART1
            Cmu::steal()
                .hfperclken0()
                .modify(|_, w| w.usart0().set_bit());
        };

        Self::SpiPart::new(usartx::<0>(), pin_clk.loc(), pin_tx.loc(), pin_rx.loc())
    }
}

impl<MCLK, MTX, MRX> UsartSpiExt<MCLK, MTX, MRX> for Usart1
where
    MCLK: OutputPin + UsartClkPin,
    MTX: OutputPin + UsartTxPin,
    MRX: InputPin + UsartRxPin,
{
    type SpiPart = Spi<1>;

    fn into_spi_bus(self, pin_clk: MCLK, pin_tx: MTX, pin_rx: MRX) -> Self::SpiPart {
        // Enable USART 1 peripheral clock
        unsafe {
            // FIXME: Hardcoded USART1
            Cmu::steal()
                .hfperclken0()
                .modify(|_, w| w.usart1().set_bit());
        };

        Self::SpiPart::new(usartx::<1>(), pin_clk.loc(), pin_tx.loc(), pin_rx.loc())
    }
}

#[derive(Debug)]
pub struct Spi<const N: u8> {
    usart: &'static RegisterBlock,
}

impl<const N: u8> Spi<N> {
    fn new(usart: &'static RegisterBlock, clk_loc: u8, tx_loc: u8, rx_loc: u8) -> Self {
        let mut spi = Spi { usart };

        spi.reset();

        spi.usart.ctrl().write(|w| {
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

        spi.usart.frame().write(|w| {
            // 8 data bits
            w.databits().eight();
            // 1 stop bit
            w.stopbits().one();
            // No parity
            w.parity().none()
        });

        // Master enable
        spi.usart.cmd().write(|w| w.masteren().set_bit());

        spi.usart.ctrl().modify(|_, w| {
            // Auto CS
            w.autocs().set_bit();
            // No CS invert
            w.csinv().clear_bit()
        });

        spi.usart.timing().modify(|_, w| {
            w.cshold().zero();
            w.cssetup().zero()
        });

        // Set IO pin routing for Usart
        spi.usart.routeloc0().modify(|_, w| unsafe {
            w.clkloc().bits(clk_loc);
            w.txloc().bits(tx_loc);
            w.rxloc().bits(rx_loc)
        });

        // Enable IO pins for Usart
        spi.usart.routepen().modify(|_, w| {
            w.clkpen().set_bit();
            w.txpen().set_bit();
            w.rxpen().set_bit()
        });

        // Finally, enable UART
        spi.usart.cmd().write(|w| {
            w.rxen().set_bit();
            w.txen().set_bit()
        });

        spi
    }

    fn reset(&mut self) {
        // Use CMD first
        self.usart.cmd().write(|w| {
            w.rxdis().set_bit();
            w.txdis().set_bit();
            w.masterdis().set_bit();
            w.rxblockdis().set_bit();
            w.txtridis().set_bit();
            w.cleartx().set_bit();
            w.clearrx().set_bit()
        });

        self.usart.ctrl().reset();
        self.usart.frame().reset();
        self.usart.trigctrl().reset();
        self.usart.clkdiv().reset();
        self.usart.ien().reset();

        // All flags for the IFC register fields
        const IFC_MASK: u32 = 0x0001FFF9;
        self.usart.ifc().write(|w| unsafe { w.bits(IFC_MASK) });

        self.usart.timing().reset();
        self.usart.routepen().reset();
        self.usart.routeloc0().reset();
        self.usart.routeloc1().reset();
        self.usart.input().reset();

        match N {
            // Only UART0 has IRDA
            0 => self.usart.irctrl().reset(),
            // Only USART1 has I2S
            1 => self.usart.i2sctrl().reset(),
            _ => unreachable!(),
        }
    }

    /// TODO:
    pub fn set_loopback(&mut self, enabled: bool) {
        self.usart.ctrl().write(|w| match enabled {
            true => w.loopbk().set_bit(),
            false => w.loopbk().clear_bit(),
        })
    }

    /// TODO:
    pub fn set_baudrate(
        &mut self,
        baudrate: HertzU32,
        clocks: &Clocks,
    ) -> Result<HertzU32, SpiError> {
        // A baudrate of 0 makes no sense
        if baudrate.raw() == 0 {
            return Err(SpiError::InvalidBaudrate(baudrate));
        }

        // Set clock divider in order to obtain the closest baudrate to the one requested. According to the reference
        // manual, the formula to calculate the Usart Clock Div is:
        //          USARTn_CLKDIV = 256 x (fHFPERCLK/(2 x brdesired) - 1)
        // We are not bitshifting by `8` (256*...) because the `div` field starts at bit 3, so we only bitshift by 5
        // let clk_div: u32 = ((clocks.hf_per_clk / (baudrate * 2)) - 1) << 5;
        let clk_div: u32 = clocks.hf_per_clk / (baudrate * 2);

        // avoid underflow if trying to subtracting `1` from a `clk_div` of `0`
        let clk_div = match clk_div {
            0 => 0,
            _ => (clk_div - 1) << 5,
        };

        self.usart
            .clkdiv()
            .write(|w| unsafe { w.div().bits(clk_div) });

        Ok(Self::calculate_baudrate(clocks.hf_per_clk, clk_div))
    }

    /// TODO:
    fn calculate_baudrate(hf_per_clk: HertzU32, clk_div: u32) -> HertzU32 {
        let divisor: u64;
        let remainder: u64;
        let quotient: u64;
        let factor: u64 = 128;
        let clk_div: u64 = (clk_div as u64) << 3;

        divisor = clk_div + 256;
        quotient = hf_per_clk.raw() as u64 / divisor;
        remainder = hf_per_clk.raw() as u64 % divisor;
        let br = (factor * quotient) as u32;
        let br = br + ((factor * remainder) / divisor) as u32;

        br.Hz()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpiError {
    InvalidBaudrate(HertzU32),
}

impl Error for SpiError {
    fn kind(&self) -> ErrorKind {
        match self {
            SpiError::InvalidBaudrate(_) => ErrorKind::Other,
        }
    }
}

// Implementations for `Pin` to be used for `embedded-hal` traits
impl<const U: u8> ErrorType for Spi<U> {
    type Error = SpiError;
}

impl<const U: u8> SpiBus<u8> for Spi<U> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        for b in words {
            // Wait for TX buffer to be empty (TXBL is set when empty)
            while self.usart.status().read().txbl().bit_is_clear() {}

            self.usart
                .txdata()
                .write(|w| unsafe { w.txdata().bits(*b) });
        }

        // Wait for TX to finish
        while self.usart.status().read().txc().bit_is_clear() {}

        Ok(())
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
