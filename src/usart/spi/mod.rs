//! Serial Peripheral Interface Bus
//!
//! Specialize USART peripherals into SPI peripherals

use crate::{
    cmu::Clocks,
    gpio::pin::{
        mode::{InputMode, OutputMode},
        Pin,
    },
    usart::{usarts::usartx, Usart},
};
use core::cmp::max;
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::{Error, ErrorKind, ErrorType, Mode, Phase, Polarity, SpiBus},
};
pub use fugit::{HertzU32, RateExtU32};

/// SPI master which implements `SpiBus` trait
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Spi<const N: u8, USART, PCLK, PTX, PRX> {
    usart: USART,
    pin_clk: PCLK,
    pin_tx: PTX,
    pin_rx: PRX,
}

impl<const N: u8, PCLK, PTX, PRX> Spi<N, Usart<N>, PCLK, PTX, PRX>
where
    PCLK: OutputPin + UsartClkPin,
    PTX: OutputPin + UsartTxPin,
    PRX: InputPin + UsartRxPin,
{
    const FILLER_BYTE: u8 = 0x00;

    pub(crate) fn new(
        usart: Usart<N>,
        pin_clk: PCLK,
        pin_tx: PTX,
        pin_rx: PRX,
        mode: Mode,
    ) -> Self {
        let mut spi = Spi {
            usart,
            pin_clk,
            pin_tx,
            pin_rx,
        };

        let usart_p = usartx::<N>();

        spi.reset();

        usart_p.ctrl().write(|w| {
            // Set USART to Synchronous Mode
            w.sync().set_bit();
            // Most significant bit first
            w.msbf().set_bit();
            // Disable auto TX
            w.autotx().clear_bit()
        });

        spi.set_mode(mode);

        usart_p.frame().write(|w| {
            // 8 data bits
            w.databits().eight();
            // 1 stop bit
            w.stopbits().one();
            // No parity
            w.parity().none()
        });

        // Master enable
        usart_p.cmd().write(|w| w.masteren().set_bit());

        usart_p.ctrl().modify(|_, w| {
            // Auto CS: a `SpiBus` implementation must not control CS pin
            w.autocs().clear_bit();
            // No CS invert
            w.csinv().clear_bit()
        });

        usart_p.timing().modify(|_, w| {
            w.cshold().zero();
            w.cssetup().zero()
        });

        // Set IO pin routing for Usart
        let clk_loc = spi.pin_clk.loc();
        let tx_loc = spi.pin_tx.loc();
        let rx_loc = spi.pin_rx.loc();
        usart_p.routeloc0().modify(|_, w| unsafe {
            w.clkloc().bits(clk_loc);
            w.txloc().bits(tx_loc);
            w.rxloc().bits(rx_loc)
        });

        // Enable IO pins for Usart
        usart_p.routepen().modify(|_, w| {
            w.clkpen().set_bit();
            w.txpen().set_bit();
            w.rxpen().set_bit()
        });

        // Enable Usart
        usart_p.cmd().write(|w| {
            w.rxen().set_bit();
            w.txen().set_bit()
        });

        spi
    }

    /// Release the resources used to create this SPI instance
    pub fn free(self) -> (Usart<N>, PCLK, PTX, PRX) {
        (self.usart, self.pin_clk, self.pin_tx, self.pin_rx)
    }

    /// Set the SPI loopback flag
    pub fn set_loopback(&mut self, enabled: bool) {
        let usart_p = usartx::<N>();
        usart_p.ctrl().write(|w| match enabled {
            true => w.loopbk().set_bit(),
            false => w.loopbk().clear_bit(),
        });
    }

    /// Set the SPI baudrate
    ///
    /// This does a best effort, so the actual calculated baudrate is returned
    pub fn set_baudrate(
        &mut self,
        baudrate: HertzU32,
        clocks: &Clocks,
    ) -> Result<HertzU32, SpiError> {
        let usart_p = usartx::<N>();

        // A baudrate of 0 makes no sense
        if baudrate.raw() == 0 {
            return Err(SpiError::InvalidBaudrate(baudrate));
        }

        // Set clock divider in order to obtain the closest baudrate to the one requested. According to the reference
        // manual, the formula to calculate the Usart Clock Div is:
        //          USARTn_CLKDIV = 256 x (fHFPERCLK/(2 x brdesired) - 1)
        // We are not bitshifting by `8` (256*...) because the `div` field starts at bit 3, so we only bitshift by 5
        // let clk_div: u32 = ((clocks.hf_per_clk / (baudrate * 2)) - 1) << 5;
        let clk_div: u32 = clocks.hf_per_clk() / (baudrate * 2);

        // avoid underflow if trying to subtracting `1` from a `clk_div` of `0`
        let clk_div = match clk_div {
            0 => 0,
            _ => (clk_div - 1) << 5,
        };

        usart_p.clkdiv().write(|w| unsafe { w.div().bits(clk_div) });

        Ok(Self::calculate_baudrate(clocks.hf_per_clk(), clk_div))
    }

    /// Set the SPI mode
    ///
    /// You can use one of the predefined [`embedded-hal`](`embedded_hal::spi::Mode`) spi modes:
    ///   - [`MODE_0`](`embedded_hal::spi::MODE_0`): CPOL = 0, CPHA = 0
    ///   - [`MODE_1`](`embedded_hal::spi::MODE_1`): CPOL = 0, CPHA = 1
    ///   - [`MODE_2`](`embedded_hal::spi::MODE_2`): CPOL = 1, CPHA = 0
    ///   - [`MODE_3`](`embedded_hal::spi::MODE_3`): CPOL = 1, CPHA = 1
    pub fn set_mode(&mut self, mode: Mode) {
        let usart_p = usartx::<N>();

        usart_p.ctrl().modify(|_, w| {
            w.clkpol()
                .bit(mode.polarity == Polarity::IdleHigh)
                .clkpha()
                .bit(mode.phase == Phase::CaptureOnSecondTransition)
        });
    }

    fn reset(&mut self) {
        let usart_p = usartx::<N>();

        // Use CMD first
        usart_p.cmd().write(|w| {
            w.rxdis().set_bit();
            w.txdis().set_bit();
            w.masterdis().set_bit();
            w.rxblockdis().set_bit();
            w.txtridis().set_bit();
            w.cleartx().set_bit();
            w.clearrx().set_bit()
        });

        usart_p.ctrl().reset();
        usart_p.frame().reset();
        usart_p.trigctrl().reset();
        usart_p.clkdiv().reset();
        usart_p.ien().reset();

        // All flags for the IFC register fields
        const IFC_MASK: u32 = 0x0001FFF9;
        usart_p.ifc().write(|w| unsafe { w.bits(IFC_MASK) });

        usart_p.timing().reset();
        usart_p.routepen().reset();
        usart_p.routeloc0().reset();
        usart_p.routeloc1().reset();
        usart_p.input().reset();

        match N {
            // Only UART0 has IRDA
            0 => usart_p.irctrl().reset(),
            // Only USART1 has I2S
            1 => usart_p.i2sctrl().reset(),
            _ => unreachable!(),
        }
    }

    /// Calculate the actual baudrate of the SPI peripheral
    fn calculate_baudrate(hf_per_clk: HertzU32, clk_div: u32) -> HertzU32 {
        let divisor: u64 = ((clk_div as u64) << 3) + 256;
        let remainder: u64 = hf_per_clk.raw() as u64 % divisor;
        let quotient: u64 = hf_per_clk.raw() as u64 / divisor;
        let factor: u64 = 128;

        let br = (factor * quotient) as u32;
        let br = br + ((factor * remainder) / divisor) as u32;

        br.Hz()
    }

    fn wait_tx_complete(&self) -> Result<(), SpiError> {
        // TODO: maybe calculate a counter based on minimum possible baudrate.
        const MAX_COUNT: u32 = 1_000_000;
        let mut bail_countdown = MAX_COUNT;
        let usart_p = usartx::<N>();

        while usart_p.status().read().txc().bit_is_clear() {
            bail_countdown -= 1;

            if bail_countdown == 0 {
                return Err(SpiError::TxUnderflow);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpiError {
    InvalidBaudrate(HertzU32),
    TxUnderflow,
    RxUnderflow,
}

impl Error for SpiError {
    fn kind(&self) -> ErrorKind {
        match self {
            SpiError::InvalidBaudrate(_) => ErrorKind::Other,
            SpiError::TxUnderflow => ErrorKind::Other,
            SpiError::RxUnderflow => ErrorKind::Other,
        }
    }
}

// Implementations for `ErrorType` to be used by `SpiBus` `embedded-hal` trait
impl<const N: u8, PCLK, PTX, PRX> ErrorType for Spi<N, Usart<N>, PCLK, PTX, PRX> {
    type Error = SpiError;
}

impl<const N: u8, PCLK, PTX, PRX> SpiBus<u8> for Spi<N, Usart<N>, PCLK, PTX, PRX>
where
    PCLK: OutputPin + UsartClkPin,
    PTX: OutputPin + UsartTxPin,
    PRX: InputPin + UsartRxPin,
{
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer(words, &[])
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        let mut words_iter = words.iter();
        let usart_p = usartx::<N>();

        // This closure  waits until there are at least 2 (out of 3) bytes available in the TX buffer
        // The first position in the TX Buffer is the Shift Register, which is not accessible through registers
        // See [Reference Manual](../../../../../doc/efm32pg1-rm.pdf#page=466)
        let wait_for_buffer_space = || {
            // TODO: maybe calculate a bailout counter based on minimum possible baudrate.
            // The current counter value was determined empirically with a requested 1Hz baudrate in *Release* build
            // (actually it's ~316 Hz, with a Peripheral clock @ 19 Mhz).
            const MAX_COUNT: u32 = 1_000_000;
            let mut bail_countdown = MAX_COUNT;

            // Wait until there are at least 2 available bytes (out of 3) in the TX buffer.
            while usart_p.status().read().txbufcnt().bits() > 1 {
                bail_countdown -= 1;

                if bail_countdown == 0 {
                    return Err(SpiError::TxUnderflow);
                }
            }
            Ok(())
        };

        while let Some(b0) = words_iter.next() {
            let usart_p = usartx::<N>();
            wait_for_buffer_space()?;

            if let Some(b1) = words_iter.next() {
                // We have 2 bytes to send, use the `txdouble` register
                usart_p.txdouble().write(|w| unsafe {
                    w.txdata0().bits(*b0);
                    w.txdata1().bits(*b1)
                });
            } else {
                // We have only 1 byte left to send, use the `txdata` register
                usart_p.txdata().write(|w| unsafe { w.txdata().bits(*b0) });
            }
        }

        Ok(())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        let max_byte_count = max(read.len(), write.len());
        let mut tx_iter = write.iter();
        let mut rx_iter = read.iter_mut();
        let mut rx_discard = 0;
        let usart_p = usartx::<N>();

        for (txo, rxo) in (0..max_byte_count).map(|_| (tx_iter.next(), rx_iter.next())) {
            let tx_byte = match txo {
                Some(txr) => *txr,
                None => Self::FILLER_BYTE,
            };

            let rx_byte = match rxo {
                Some(rx) => rx,
                None => &mut rx_discard,
            };

            usart_p
                .txdata()
                .write(|w| unsafe { w.txdata().bits(tx_byte) });

            self.wait_tx_complete()?;

            *rx_byte = usart_p.rxdata().read().rxdata().bits();
        }

        Ok(())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        let mut words_iter = words.iter_mut();

        while let Some(b0) = words_iter.next() {
            let usart_p = usartx::<N>();

            if let Some(b1) = words_iter.next() {
                // We have 2 bytes to send, use the `txdouble` register
                usart_p.txdouble().write(|w| unsafe {
                    w.txdata0().bits(*b0);
                    w.txdata1().bits(*b1)
                });

                self.wait_tx_complete()?;

                *b0 = usart_p.rxdouble().read().rxdata0().bits();
                *b1 = usart_p.rxdouble().read().rxdata1().bits();
            } else {
                // We have only 1 byte left to send, use the `txdata` register
                usart_p.txdata().write(|w| unsafe { w.txdata().bits(*b0) });

                self.wait_tx_complete()?;

                *b0 = usart_p.rxdata().read().rxdata().bits();
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.wait_tx_complete()
    }
}

/// Marker trait to enforce which (output) pins can be used as an SPI Clock output.
///
/// This trait is implemented privately in this module for select pins specified in the
/// [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85), and it is used to constrain the type of the `pin_clk`
/// parameter passed to the `into_spi_bus()` method of the `UsartSpiExt` trait.
///
/// Note: if you try to create an `Spi` instance and get a compiler error like
/// ```text
///     the trait `efm32pg1b_hal::spi::UsartClkPin` is not implemented for
///     `efm32pg1b_hal::gpio::Pin<'D', 8, efm32pg1b_hal::gpio::Input>`, which is required by
///     `efm32pg1b_hal::efm32pg1b_pac::Usart1: efm32pg1b_hal::spi::UsartSpiExt<_, _, _>`
/// ```
///
/// then it's probably the case that you're trying to use a Pin as an SPI Clock pin when that pin is not available
/// to the `usart` peripheral as a CLK pin.
///
/// Please consult the [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85) (`US0_CLK` or `US1_CLK` Alternate
/// Functionality) to see which pins can be used as SPI clock pins.
pub trait UsartClkPin {
    fn loc(&self) -> u8;
}

/// Implement the `UsartClkPin` trait for the `US0_CLK`/`US1_CLK` alternate function.
/// See [Data Sheet](../../../../../doc/efm32pg1-datasheet.pdf#page=86).
macro_rules! impl_clock_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<MODE> UsartClkPin for Pin<$port, $pin, MODE>
        where
            MODE: OutputMode,
        {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_clock_loc!(0, 'A', 2);
impl_clock_loc!(1, 'A', 3);
impl_clock_loc!(2, 'A', 4);
impl_clock_loc!(3, 'A', 5);
impl_clock_loc!(4, 'B', 11);
impl_clock_loc!(5, 'B', 12);
impl_clock_loc!(6, 'B', 13);
impl_clock_loc!(7, 'B', 14);
impl_clock_loc!(8, 'B', 15);
impl_clock_loc!(9, 'C', 6);
impl_clock_loc!(10, 'C', 7);
impl_clock_loc!(11, 'C', 8);
impl_clock_loc!(12, 'C', 9);
impl_clock_loc!(13, 'C', 10);
impl_clock_loc!(14, 'C', 11);
impl_clock_loc!(15, 'D', 9);
impl_clock_loc!(16, 'D', 10);
impl_clock_loc!(17, 'D', 11);
impl_clock_loc!(18, 'D', 12);
impl_clock_loc!(19, 'D', 13);
impl_clock_loc!(20, 'D', 14);
impl_clock_loc!(21, 'D', 15);
impl_clock_loc!(22, 'F', 0);
impl_clock_loc!(23, 'F', 1);
impl_clock_loc!(24, 'F', 2);
impl_clock_loc!(25, 'F', 3);
impl_clock_loc!(26, 'F', 4);
impl_clock_loc!(27, 'F', 5);
impl_clock_loc!(28, 'F', 6);
impl_clock_loc!(29, 'F', 7);
impl_clock_loc!(30, 'A', 0);
impl_clock_loc!(31, 'A', 1);

/// Marker trait to enforce which (output) pins can be used as an SPI Tx output.
///
/// This trait is implemented privately in this module for select pins specified in the
/// [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85), and it is used to constrain the type of the `pin_tx`
/// parameter passed to the `into_spi_bus()` method of the `UsartSpiExt` trait.
///
/// Note: if you try to create an `Spi` instance and get a compiler error like
/// ```text
///     the trait `efm32pg1b_hal::spi::UsartTxPin` is not implemented for
///     `efm32pg1b_hal::gpio::Pin<'D', 8, efm32pg1b_hal::gpio::Input>`, which is required by
///     `efm32pg1b_hal::efm32pg1b_pac::Usart1: efm32pg1b_hal::spi::UsartSpiExt<_, _, _>`
/// ```
///
/// then it's probably the case that you're trying to use a Pin as an SPI Tx pin when that pin is not available
/// to the `usart` peripheral as a TX pin.
///
/// Please consult the [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85) (`US0_TX` or `US1_TX` Alternate
/// Functionality) to see which pins can be used as SPI Tx pins.
pub trait UsartTxPin {
    fn loc(&self) -> u8;
}

/// Implement the `UsartTxPin` trait for the `US0_TX`/`US1_TX` alternate function.
/// See [Data Sheet](../../../../../doc/efm32pg1-datasheet.pdf#page=86).
macro_rules! impl_tx_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<MODE> UsartTxPin for Pin<$port, $pin, MODE>
        where
            MODE: OutputMode,
        {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_tx_loc!(0, 'A', 0);
impl_tx_loc!(1, 'A', 1);
impl_tx_loc!(2, 'A', 2);
impl_tx_loc!(3, 'A', 3);
impl_tx_loc!(4, 'A', 4);
impl_tx_loc!(5, 'A', 5);
impl_tx_loc!(6, 'B', 11);
impl_tx_loc!(7, 'B', 12);
impl_tx_loc!(8, 'B', 13);
impl_tx_loc!(9, 'B', 14);
impl_tx_loc!(10, 'B', 15);
impl_tx_loc!(11, 'C', 6);
impl_tx_loc!(12, 'C', 7);
impl_tx_loc!(13, 'C', 8);
impl_tx_loc!(14, 'C', 9);
impl_tx_loc!(15, 'C', 10);
impl_tx_loc!(16, 'C', 11);
impl_tx_loc!(17, 'D', 9);
impl_tx_loc!(18, 'D', 10);
impl_tx_loc!(19, 'D', 11);
impl_tx_loc!(20, 'D', 12);
impl_tx_loc!(21, 'D', 13);
impl_tx_loc!(22, 'D', 14);
impl_tx_loc!(23, 'D', 15);
impl_tx_loc!(24, 'F', 0);
impl_tx_loc!(25, 'F', 1);
impl_tx_loc!(26, 'F', 2);
impl_tx_loc!(27, 'F', 3);
impl_tx_loc!(28, 'F', 4);
impl_tx_loc!(29, 'F', 5);
impl_tx_loc!(30, 'F', 6);
impl_tx_loc!(31, 'F', 7);

/// Marker trait to enforce which (input) pins can be used as an SPI Rx input.
///
/// This trait is implemented privately in this module for select pins specified in the
/// [Data Sheet - page 86](../../../../../doc/efm32pg1-datasheet.pdf#page=86), and it is used to constrain the type of the `pin_rx`
/// parameter passed to the `into_spi_bus()` method of the `UsartSpiExt` trait.
///
/// Note: if you try to create an `Spi` instance and get a compiler error like
/// ```sh
///     the trait `efm32pg1b_hal::spi::UsartRxPin` is not implemented for
///     `efm32pg1b_hal::gpio::Pin<'D', 8, efm32pg1b_hal::gpio::Input>`, which is required by
///     `efm32pg1b_hal::efm32pg1b_pac::Usart1: efm32pg1b_hal::spi::UsartSpiExt<_, _, _>`
/// ```
///
/// then it's probably the case that you're trying to use a Pin as an SPI Rx pin when that pin is not available
/// to the `usart` peripheral as a RX pin.
///
/// Please consult the [Data Sheet - page 86](../../../../../doc/efm32pg1-datasheet.pdf#page=86) (`US0_RX` or `US1_RX` Alternate
/// Functionality) to see which pins can be used as SPI Rx pins.
pub trait UsartRxPin {
    fn loc(&self) -> u8;
}

/// Implement the `UsartRxkPin` trait for the `US0_RX`/`US1_RX` alternate function.
/// See [Data Sheet](../../../../../doc/efm32pg1-datasheet.pdf#page=86).
macro_rules! impl_rx_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<MODE> UsartRxPin for Pin<$port, $pin, MODE>
        where
            MODE: InputMode,
        {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_rx_loc!(0, 'A', 1);
impl_rx_loc!(1, 'A', 2);
impl_rx_loc!(2, 'A', 3);
impl_rx_loc!(3, 'A', 4);
impl_rx_loc!(4, 'A', 5);
impl_rx_loc!(5, 'B', 11);
impl_rx_loc!(6, 'B', 12);
impl_rx_loc!(7, 'B', 13);
impl_rx_loc!(8, 'B', 14);
impl_rx_loc!(9, 'B', 15);
impl_rx_loc!(10, 'C', 6);
impl_rx_loc!(11, 'C', 7);
impl_rx_loc!(12, 'C', 8);
impl_rx_loc!(13, 'C', 9);
impl_rx_loc!(14, 'C', 10);
impl_rx_loc!(15, 'C', 11);
impl_rx_loc!(16, 'D', 9);
impl_rx_loc!(17, 'D', 10);
impl_rx_loc!(18, 'D', 11);
impl_rx_loc!(19, 'D', 12);
impl_rx_loc!(20, 'D', 13);
impl_rx_loc!(21, 'D', 14);
impl_rx_loc!(22, 'D', 15);
impl_rx_loc!(23, 'F', 0);
impl_rx_loc!(24, 'F', 1);
impl_rx_loc!(25, 'F', 2);
impl_rx_loc!(26, 'F', 3);
impl_rx_loc!(27, 'F', 4);
impl_rx_loc!(28, 'F', 5);
impl_rx_loc!(29, 'F', 6);
impl_rx_loc!(30, 'F', 7);
impl_rx_loc!(31, 'A', 0);

/// Marker trait to enforce which (output) pins can be used as an SPI CS output.
///
/// TODO: this is not actually used when instantiating an SPI. Should it?
///
/// Please consult the [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85) (`US0_CS` or `US1_CS` Alternate
/// Functionality) to see which pins can be used as SPI CS pins.
pub trait UsartCsPin {
    fn loc(&self) -> u8;
}

/// Implement the `UsartCsPin` trait for the `US0_CS`/`US1_CS` alternate function.
/// See [Data Sheet](../../../../../doc/efm32pg1-datasheet.pdf#page=86).
macro_rules! impl_cs_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<MODE> UsartCsPin for Pin<$port, $pin, MODE>
        where
            MODE: OutputMode,
        {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_cs_loc!(0, 'A', 3);
impl_cs_loc!(1, 'A', 4);
impl_cs_loc!(2, 'A', 5);
impl_cs_loc!(3, 'B', 11);
impl_cs_loc!(4, 'B', 12);
impl_cs_loc!(5, 'B', 13);
impl_cs_loc!(6, 'B', 14);
impl_cs_loc!(7, 'B', 15);
impl_cs_loc!(8, 'C', 6);
impl_cs_loc!(9, 'C', 7);
impl_cs_loc!(10, 'C', 8);
impl_cs_loc!(11, 'C', 9);
impl_cs_loc!(12, 'C', 10);
impl_cs_loc!(13, 'C', 11);
impl_cs_loc!(14, 'D', 9);
impl_cs_loc!(15, 'D', 10);
impl_cs_loc!(16, 'D', 11);
impl_cs_loc!(17, 'D', 12);
impl_cs_loc!(18, 'D', 13);
impl_cs_loc!(19, 'D', 14);
impl_cs_loc!(20, 'D', 15);
impl_cs_loc!(21, 'F', 0);
impl_cs_loc!(22, 'F', 1);
impl_cs_loc!(23, 'F', 2);
impl_cs_loc!(24, 'F', 3);
impl_cs_loc!(25, 'F', 4);
impl_cs_loc!(26, 'F', 5);
impl_cs_loc!(27, 'F', 6);
impl_cs_loc!(28, 'F', 7);
impl_cs_loc!(29, 'A', 0);
impl_cs_loc!(30, 'A', 1);
impl_cs_loc!(31, 'A', 2);
