//! Serial Peripheral Interface Bus
//!
//! Specialize USART peripherals into SPI peripherals

pub mod dma;
#[cfg(feature = "efemb")]
pub mod efemb;

use crate::{
    dma::{DmaChannel, DmaError},
    gpio::{
        dynamic::DynamicPin,
        pin::{
            mode::{InputMode, OutputMode},
            Pin, PinId, PinInfo,
        },
        port::PortId,
    },
    usart::{mmio, spi::dma::SpiDma, UsartId, UsartIndex},
};
use core::cmp::max;
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::{Error, ErrorKind, ErrorType, Mode, Phase, Polarity, SpiBus},
};

/// SPI filler byte
///
/// Used when the SPI only needs to receive, in which case it will clock out this byte on MOSI
pub const TX_FILLER_BYTE: u8 = 0xFF;

/// SPI master which implements `SpiBus` trait
///
/// This driver is fully non-generic: the USART peripheral is selected at runtime (via
/// [`UsartId`]) and the pins are stored in their type-erased [`DynamicPin`] form. All build-time
/// validity (which pins may serve as CLK/TX/RX, and which USART is used) is enforced at compile
/// time by the generic [`SpiPins`] builder, and the SPI operating parameters are supplied via the
/// non-generic [`Config`]. The only way to obtain an `Spi` is through a valid `SpiPins` + `Config`
/// passed to [`Spi::new`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Spi {
    /// USART peripheral this instance drives
    id: UsartId,
    pin_clk: DynamicPin,
    pin_tx: DynamicPin,
    pin_rx: DynamicPin,
}

impl Spi {
    /// Create a new SPI instance from a validated [`SpiPins`] pin/peripheral binding and an
    /// initial [`Config`].
    ///
    /// The USART peripheral and pin routing are taken from `pins`, and the SPI [`Mode`],
    /// [`BitOrder`], loopback flag, sample delay and baudrate divider are all applied from
    /// `config` (via [`Spi::set_config`]). The returned [`Spi`] is non-generic: the peripheral
    /// is stored at runtime as a [`UsartId`] and the pins are held in their type-erased
    /// [`DynamicPin`] form.
    pub fn new(pins: SpiPins, config: &Config) -> Self {
        let id = pins.id;

        // Enable the clock for this USART
        mmio::cmu_usart_enable(id);
        // Reset the USART registers
        mmio::reset(id);

        let mut spi = Spi {
            id,
            pin_clk: pins.pin_clk,
            pin_tx: pins.pin_tx,
            pin_rx: pins.pin_rx,
        };

        let usart_p = mmio::usartx(id);

        spi.reset();

        usart_p.ctrl().write(|w| {
            // Set USART to Synchronous Mode
            w.sync().set_bit();
            // Most significant bit first (the exact bit order is re-applied from `config` below)
            w.msbf().set_bit();
            // Disable auto TX
            w.autotx().clear_bit()
        });

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
        usart_p.routeloc0().modify(|_, w| unsafe {
            w.clkloc().bits(pins.clk_loc);
            w.txloc().bits(pins.tx_loc);
            w.rxloc().bits(pins.rx_loc)
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

        // Apply the SPI operating configuration (mode, bit order, loopback, sample delay,
        // baudrate divider). This is the same path as `set_config`, so the initial state of the
        // driver matches a subsequent runtime reconfiguration.
        spi.set_config(config);

        spi
    }

    /// Release the resources used to create this SPI instance
    /// FIXME: return the usart peripheral too
    pub fn release(self) -> (DynamicPin, DynamicPin, DynamicPin) {
        (self.pin_clk, self.pin_tx, self.pin_rx)
    }

    /// Returns the [`UsartId`] of the USART peripheral this instance drives.
    pub fn id(&self) -> UsartId {
        self.id
    }

    /// Set the SPI loopback flag
    ///
    /// Only the loopback bit of `CTRL` is touched; the rest of the register (synchronous mode,
    /// bit order, SPI mode, auto-TX, auto-CS, ...) is preserved.
    pub fn set_loopback(&mut self, enabled: bool) {
        let usart_p = mmio::usartx(self.id);
        usart_p.ctrl().modify(|_, w| match enabled {
            true => w.loopbk().set_bit(),
            false => w.loopbk().clear_bit(),
        });
    }

    /// Set the SPI clock divider ratio `N`.
    ///
    /// The SPI clock becomes `fHFPERCLK / (2 * (N + 1))`. The driver programs `N` directly into
    /// the `USARTn_CLKDIV.DIV` field (the register value is `N << 5`, since the field starts at
    /// bit 3). `N = 0` selects the maximum baudrate (`fHFPERCLK / 2`).
    ///
    /// See [Reference Manual - 16.5.6](../../../../../doc/efm32pg1-rm.pdf#page=506).
    pub fn set_divider(&mut self, divider: u32) {
        let usart_p = mmio::usartx(self.id);

        // The `div` field starts at bit 3, so the register value is `divider << 5`
        // (equivalent to `256 * (fHFPERCLK/(2 * fbr) - 1)` per the reference manual, since the
        // field value is CLKDIV/8 and 256 = 2^8 -> shift by 8 - 3 = 5).
        let clk_div = divider << 5;

        usart_p.clkdiv().write(|w| unsafe { w.div().bits(clk_div) });
    }

    /// Set the SPI mode
    ///
    /// You can use one of the predefined [`embedded-hal`](`embedded_hal::spi::Mode`) spi modes:
    ///   - [`MODE_0`](`embedded_hal::spi::MODE_0`): CPOL = 0, CPHA = 0
    ///   - [`MODE_1`](`embedded_hal::spi::MODE_1`): CPOL = 0, CPHA = 1
    ///   - [`MODE_2`](`embedded_hal::spi::MODE_2`): CPOL = 1, CPHA = 0
    ///   - [`MODE_3`](`embedded_hal::spi::MODE_3`): CPOL = 1, CPHA = 1
    pub fn set_mode(&mut self, mode: Mode) {
        let usart_p = mmio::usartx(self.id);

        usart_p.ctrl().modify(|_, w| {
            w.clkpol()
                .bit(mode.polarity == Polarity::IdleHigh)
                .clkpha()
                .bit(mode.phase == Phase::CaptureOnSecondTransition)
        });
    }

    /// Set the SPI bit order via the `USARTn_CTRL.MSBF` field.
    ///
    /// See [Reference Manual - 16.5.1](../../../../../doc/efm32pg1-rm.pdf#page=494).
    pub fn set_bit_order(&mut self, bit_order: BitOrder) {
        let usart_p = mmio::usartx(self.id);
        usart_p.ctrl().modify(|_, w| match bit_order {
            BitOrder::LsbFirst => w.msbf().clear_bit(),
            BitOrder::MsbFirst => w.msbf().set_bit(),
        });
    }

    /// Enable or disable the synchronous-master sample delay (`USARTn_CTRL.SMSDELAY`).
    ///
    /// When enabled, the master sample point is delayed to the next setup edge, which can improve
    /// timing margin and allow higher speeds with some slaves. See
    /// [Reference Manual - 16.5.1](../../../../../doc/efm32pg1-rm.pdf#page=494).
    pub fn set_sms_delay(&mut self, enabled: bool) {
        let usart_p = mmio::usartx(self.id);
        usart_p.ctrl().modify(|_, w| match enabled {
            true => w.smsdelay().set_bit(),
            false => w.smsdelay().clear_bit(),
        });
    }

    /// Apply a [`Config`] to this driver at runtime.
    ///
    /// This reconfigures the SPI [`Mode`], [`BitOrder`], loopback flag, synchronous-master sample
    /// delay and baudrate divider in place, without rebuilding the driver. The USART peripheral
    /// and the pin routing remain fixed (they are bound at build time via [`SpiPins`]).
    pub fn set_config(&mut self, config: &Config) {
        self.set_mode(config.mode);
        self.set_bit_order(config.bit_order);
        self.set_loopback(config.loopback);
        self.set_sms_delay(config.sms_delay);
        self.set_divider(config.divider);
    }

    /// Convert into a Spi implementation which used DMA channels
    pub fn into_spi_dma(self, tx: DmaChannel, rx: DmaChannel) -> SpiDma {
        SpiDma::new(self, tx, rx)
    }

    fn reset(&mut self) {
        let usart_p = mmio::usartx(self.id);

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

        match self.id {
            // Only USART0 has IrDA
            UsartId::Usart0 => usart_p.irctrl().reset(),
            // Only USART1 has I2S
            UsartId::Usart1 => usart_p.i2sctrl().reset(),
        }
    }

    fn wait_tx_complete(&self) -> Result<(), SpiError> {
        // TODO: maybe calculate a counter based on minimum possible baudrate.
        const MAX_COUNT: u32 = 1_000_000;
        let mut bail_countdown = MAX_COUNT;
        let usart_p = mmio::usartx(self.id);

        while usart_p.status().read().txc().bit_is_clear() {
            bail_countdown -= 1;

            if bail_countdown == 0 {
                return Err(SpiError::TxUnderflow);
            }
        }
        Ok(())
    }
}

/// The USART peripheral and CLK/TX/RX pins an [`Spi`] driver is built from.
///
/// `SpiPins` is constructed generically via [`SpiPins::new`], which enforces at compile time
/// that:
///   - `USART` is a valid USART peripheral ([`UsartIndex`]),
///   - `PCLK` is a pin usable as the SPI clock output ([`UsartClkPin`]),
///   - `PTX` is a pin usable as the SPI MOSI/TX output ([`UsartTxPin`]),
///   - `PRX` is a pin usable as the SPI MISO/RX input ([`UsartRxPin`]).
///
/// `SpiPins::new` resolves the [`UsartId`] and erases the pins into [`DynamicPin`]s (extracting
/// the routing locations first), so the resulting `SpiPins` is fully non-generic. `Spi::new`
/// takes it with no trait bounds.
///
/// `SpiPins` only carries the peripheral and pin routing; the SPI operating mode, baudrate, bit
/// order, loopback and sample delay are all configured via the [`Config`] passed to [`Spi::new`].
#[derive(Debug)]
pub struct SpiPins {
    id: UsartId,
    pin_clk: DynamicPin,
    pin_tx: DynamicPin,
    pin_rx: DynamicPin,
    clk_loc: u8,
    tx_loc: u8,
    rx_loc: u8,
}

impl SpiPins {
    /// Collect the USART peripheral and its CLK/TX/RX pins for an [`Spi`] driver.
    ///
    /// The trait bounds guarantee that only pin types valid for the chosen USART are accepted,
    /// so the returned `SpiPins` always represents a valid pin/peripheral combination. The pins
    /// are type-erased into [`DynamicPin`]s and the [`UsartId`] is resolved here, so the returned
    /// `SpiPins` is non-generic. SPI operating parameters (mode, baudrate, ...) are supplied
    /// separately via [`Config`] to [`Spi::new`].
    pub fn new<USART, PCLK, PTX, PRX>(
        _usart: USART,
        pin_clk: PCLK,
        pin_tx: PTX,
        pin_rx: PRX,
    ) -> Self
    where
        USART: UsartIndex,
        PCLK: OutputPin + UsartClkPin + PinInfo,
        PTX: OutputPin + UsartTxPin + PinInfo,
        PRX: InputPin + UsartRxPin + PinInfo,
    {
        // Extract the routing locations before erasing, since `UsartClkPin`/`UsartTxPin`/
        // `UsartRxPin` are only implemented for `Pin` types.
        let clk_loc = pin_clk.loc();
        let tx_loc = pin_tx.loc();
        let rx_loc = pin_rx.loc();

        Self {
            id: USART::index(),
            pin_clk: DynamicPin::new(pin_clk.port(), pin_clk.pin(), pin_clk.mode()),
            pin_tx: DynamicPin::new(pin_tx.port(), pin_tx.pin(), pin_tx.mode()),
            pin_rx: DynamicPin::new(pin_rx.port(), pin_rx.pin(), pin_rx.mode()),
            clk_loc,
            tx_loc,
            rx_loc,
        }
    }

    /// Collect the USART peripheral and its CLK/TX/RX pins for an [`Spi`] driver, using
    /// already type-erased [`DynamicPin`]s.
    ///
    /// Unlike [`SpiPins::new`], which enforces pin/role validity at compile time via trait
    /// bounds, `try_new` validates at runtime that:
    ///   - the CLK and TX pins are valid SPI clock/TX pins and are in an output mode,
    ///   - the RX pin is a valid SPI RX pin and is in an input mode.
    ///
    /// Returns [`Err(SpiError::InvalidPin)`] if any pin is invalid for its role or in the wrong
    /// mode. The USART peripheral is specified by [`UsartId`] (runtime, not generic).
    pub fn try_new(
        id: UsartId,
        pin_clk: DynamicPin,
        pin_tx: DynamicPin,
        pin_rx: DynamicPin,
    ) -> Result<Self, SpiError> {
        let clk_loc = clk_loc(pin_clk.port(), pin_clk.pin()).ok_or(SpiError::InvalidPin)?;
        let tx_loc = tx_loc(pin_tx.port(), pin_tx.pin()).ok_or(SpiError::InvalidPin)?;
        let rx_loc = rx_loc(pin_rx.port(), pin_rx.pin()).ok_or(SpiError::InvalidPin)?;

        // CLK and TX must be output pins; RX must be an input pin.
        if !pin_clk.mode().writable() || !pin_tx.mode().writable() {
            return Err(SpiError::InvalidPin);
        }
        if !pin_rx.mode().readable_input() {
            return Err(SpiError::InvalidPin);
        }

        Ok(Self {
            id,
            pin_clk,
            pin_tx,
            pin_rx,
            clk_loc,
            tx_loc,
            rx_loc,
        })
    }
}

/// SPI bit order, selected via the `USARTn_CTRL.MSBF` field.
///
/// See [Reference Manual - 16.5.1](../../../../../doc/efm32pg1-rm.pdf#page=494).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BitOrder {
    /// Least-significant bit first (`MSBF = 0`).
    ///
    /// This is the USART's reset default and matches most SPI slaves' expectation.
    LsbFirst,
    /// Most-significant bit first (`MSBF = 1`).
    ///
    /// Useful for devices whose datasheet specifies MSB-first framing.
    #[default]
    MsbFirst,
}

/// SPI operating configuration for an [`Spi`] driver.
///
/// `Config` carries the settings that can be changed on an existing driver via
/// [`Spi::set_config`]:
///   - the SPI [`Mode`] (CPOL/CPHA),
///   - the baudrate divider `N`,
///   - the [`BitOrder`],
///   - the loopback flag (debug/test; off by default),
///   - the synchronous-master sample delay (`SMSDELAY`; off by default).
///
/// The baudrate is specified as a *divider ratio* `N` rather than a target frequency: the SPI
/// clock becomes `fHFPERCLK / (2 * (N + 1))`, and `N` is programmed directly into the
/// `USARTn_CLKDIV.DIV` field. This matches the reference manual formula
/// `USARTn_CLKDIV = 256 * (fHFPERCLK/(2 * fbr) - 1)`, where `N = fHFPERCLK/(2 * fbr) - 1`. See
/// [Reference Manual - 16.5.6](../../../../../doc/efm32pg1-rm.pdf#page=506).
///
/// Every field has a sane default (see [`Config::default`]), so a `Config` is always valid by
/// construction: every `u32` is a valid divider (`0` selects the maximum baudrate), and the
/// remaining fields are enums/bools with no invalid representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    mode: Mode,
    divider: u32,
    bit_order: BitOrder,
    loopback: bool,
    sms_delay: bool,
}

impl Default for Config {
    /// A conservative default: [`MODE_0`], maximum baudrate (`N = 0`),
    /// [`BitOrder::MsbFirst`], loopback disabled, sample delay disabled.
    ///
    /// [`MODE_0`]: embedded_hal::spi::MODE_0
    fn default() -> Self {
        Self {
            mode: embedded_hal::spi::MODE_0,
            divider: 0,
            bit_order: BitOrder::MsbFirst,
            loopback: false,
            sms_delay: false,
        }
    }
}

impl Config {
    /// Create a new `Config` with the given [`Mode`] and baudrate divider `N`.
    ///
    /// The resulting SPI clock is `fHFPERCLK / (2 * (N + 1))`. `N = 0` selects the maximum
    /// baudrate. All other fields take their [`Config::default`] values (MSB-first, loopback
    /// off, sample delay off).
    pub fn new(mode: Mode, divider: u32) -> Self {
        Self {
            mode,
            divider,
            ..Self::default()
        }
    }

    /// Set the SPI [`Mode`] of this configuration.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the baudrate divider `N` of this configuration.
    ///
    /// The resulting SPI clock is `fHFPERCLK / (2 * (N + 1))`.
    pub fn with_divider(mut self, divider: u32) -> Self {
        self.divider = divider;
        self
    }

    /// Set the [`BitOrder`] of this configuration.
    pub fn with_bit_order(mut self, bit_order: BitOrder) -> Self {
        self.bit_order = bit_order;
        self
    }

    /// Enable or disable loopback mode in this configuration.
    pub fn with_loopback(mut self, enabled: bool) -> Self {
        self.loopback = enabled;
        self
    }

    /// Enable or disable the synchronous-master sample delay (`SMSDELAY`).
    ///
    /// When set, the master sample point is delayed to the next setup edge, which can improve
    /// timing margin and allow communication at higher speeds with some slaves. See
    /// [Reference Manual - 16.5.1](../../../../../doc/efm32pg1-rm.pdf#page=494).
    pub fn with_sms_delay(mut self, enabled: bool) -> Self {
        self.sms_delay = enabled;
        self
    }
}

/// SPI Errors
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpiError {
    /// A pin passed to `SpiPins::try_new` is not valid for its SPI role (CLK/TX/RX), or is in
    /// the wrong mode (CLK/TX must be an output, RX must be an input).
    InvalidPin,
    /// Tx underflow
    TxUnderflow,
    /// Rx underflow
    RxUnderflow,
    /// SPI peripheral is busy
    Busy,
    /// TX Error
    Tx,
    /// RX Error
    Rx,
    /// TX and RX Error
    TxRx,
    /// SPI DMA error
    Dma(DmaError),
}

impl From<DmaError> for SpiError {
    fn from(value: DmaError) -> Self {
        Self::Dma(value)
    }
}

impl Error for SpiError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

// Implementations for `ErrorType` to be used by `SpiBus` `embedded-hal` trait
impl ErrorType for Spi {
    type Error = SpiError;
}

impl SpiBus<u8> for Spi {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer(words, &[])
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        let mut words_iter = words.iter();
        let usart_p = mmio::usartx(self.id);

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
        let usart_p = mmio::usartx(self.id);

        for (txo, rxo) in (0..max_byte_count).map(|_| (tx_iter.next(), rx_iter.next())) {
            let tx_byte = match txo {
                Some(txr) => *txr,
                None => TX_FILLER_BYTE,
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
        let usart_p = mmio::usartx(self.id);

        while let Some(b0) = words_iter.next() {
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
/// parameter passed to the `Config::new()` function used to build an `Spi` driver.
///
/// Note: if you try to create an `Spi` instance and get a compiler error like
/// ```text
///     the trait `efm32pg1b_hal::spi::UsartClkPin` is not implemented for
///     `efm32pg1b_hal::gpio::Pin<'D', 8, efm32pg1b_hal::gpio::Input>`, which is required by
///     `efm32pg1b_hal::usart::spi::Spi::new<efm32pg1b_hal::efm32pg1b_pac::Usart1, _, _, _>`
/// ```
///
/// then it's probably the case that you're trying to use a Pin as an SPI Clock pin when that pin is not available
/// to the `usart` peripheral as a CLK pin.
///
/// Please consult the [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85) (`US0_CLK` or `US1_CLK` Alternate
/// Functionality) to see which pins can be used as SPI clock pins.
pub trait UsartClkPin {
    /// Value to be written to USARTn_ROUTELOC0 to select the pin which wil function as the CLK pin
    /// `Pin` types which can function as CLK pins will implement this trait
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
/// parameter passed to the `Config::new()` function used to build an `Spi` driver.
///
/// Note: if you try to create an `Spi` instance and get a compiler error like
/// ```text
///     the trait `efm32pg1b_hal::spi::UsartTxPin` is not implemented for
///     `efm32pg1b_hal::gpio::Pin<'D', 8, efm32pg1b_hal::gpio::Input>`, which is required by
///     `efm32pg1b_hal::usart::spi::Spi::new<efm32pg1b_hal::efm32pg1b_pac::Usart1, _, _, _>`
/// ```
///
/// then it's probably the case that you're trying to use a Pin as an SPI Tx pin when that pin is not available
/// to the `usart` peripheral as a TX pin.
///
/// Please consult the [Data Sheet - page 85](../../../../../doc/efm32pg1-datasheet.pdf#page=85) (`US0_TX` or `US1_TX` Alternate
/// Functionality) to see which pins can be used as SPI Tx pins.
pub trait UsartTxPin {
    /// Value to be written to USARTn_ROUTELOC0 to select the pin which wil function as the TX pin
    /// `Pin` types which can function as TX pins will implement this trait
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
/// parameter passed to the `Config::new()` function used to build an `Spi` driver.
///
/// Note: if you try to create an `Spi` instance and get a compiler error like
/// ```sh
///     the trait `efm32pg1b_hal::spi::UsartRxPin` is not implemented for
///     `efm32pg1b_hal::gpio::Pin<'D', 8, efm32pg1b_hal::gpio::Input>`, which is required by
///     `efm32pg1b_hal::usart::spi::Spi::new<efm32pg1b_hal::efm32pg1b_pac::Usart1, _, _, _>`
/// ```
///
/// then it's probably the case that you're trying to use a Pin as an SPI Rx pin when that pin is not available
/// to the `usart` peripheral as a RX pin.
///
/// Please consult the [Data Sheet - page 86](../../../../../doc/efm32pg1-datasheet.pdf#page=86) (`US0_RX` or `US1_RX` Alternate
/// Functionality) to see which pins can be used as SPI Rx pins.
pub trait UsartRxPin {
    /// Value to be written to USARTn_ROUTELOC0 to select the pin which wil function as the RX pin
    /// `Pin` types which can function as RX pins will implement this trait
    fn loc(&self) -> u8;
}

/// Implement the `UsartRxPin` trait for the `US0_RX`/`US1_RX` alternate function.
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
    /// Value to be written to USARTn_ROUTELOC0 to select the pin which wil function as the CS/SS pin
    /// `Pin` types which can function as CS/SS pins will implement this trait
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

/// Resolve the `ROUTELOC0` value for a CLK pin at runtime, or `None` if the pin is not a valid
/// SPI clock pin. Mirrors the `impl_clock_loc!` macro table.
const fn clk_loc(port: PortId, pin: PinId) -> Option<u8> {
    let pin = pin as u8;
    match port {
        PortId::A if pin >= 2 && pin <= 5 => Some(pin - 2),
        PortId::A if pin <= 1 => Some(30 + pin),
        PortId::B if pin >= 11 && pin <= 15 => Some(4 + (pin - 11)),
        PortId::C if pin >= 6 && pin <= 11 => Some(9 + (pin - 6)),
        PortId::D if pin >= 9 && pin <= 15 => Some(15 + (pin - 9)),
        PortId::F if pin <= 7 => Some(22 + pin),
        _ => None,
    }
}

/// Resolve the `ROUTELOC0` value for a TX pin at runtime, or `None` if the pin is not a valid
/// SPI TX pin. Mirrors the `impl_tx_loc!` macro table.
const fn tx_loc(port: PortId, pin: PinId) -> Option<u8> {
    let pin = pin as u8;
    match port {
        PortId::A if pin <= 5 => Some(pin),
        PortId::B if pin >= 11 && pin <= 15 => Some(6 + (pin - 11)),
        PortId::C if pin >= 6 && pin <= 11 => Some(11 + (pin - 6)),
        PortId::D if pin >= 9 && pin <= 15 => Some(17 + (pin - 9)),
        PortId::F if pin <= 7 => Some(24 + pin),
        _ => None,
    }
}

/// Resolve the `ROUTELOC0` value for an RX pin at runtime, or `None` if the pin is not a valid
/// SPI RX pin. Mirrors the `impl_rx_loc!` macro table.
const fn rx_loc(port: PortId, pin: PinId) -> Option<u8> {
    let pin = pin as u8;
    match port {
        PortId::A if pin >= 1 && pin <= 5 => Some(pin - 1),
        PortId::A if pin == 0 => Some(31),
        PortId::B if pin >= 11 && pin <= 15 => Some(5 + (pin - 11)),
        PortId::C if pin >= 6 && pin <= 11 => Some(10 + (pin - 6)),
        PortId::D if pin >= 9 && pin <= 15 => Some(16 + (pin - 9)),
        PortId::F if pin <= 7 => Some(23 + pin),
        _ => None,
    }
}

/// Resolve the `ROUTELOC0` value for a CS pin at runtime, or `None` if the pin is not a valid
/// SPI CS pin. Mirrors the `impl_cs_loc!` macro table.
const fn cs_loc(port: PortId, pin: PinId) -> Option<u8> {
    let pin = pin as u8;
    match port {
        PortId::A if pin >= 3 && pin <= 5 => Some(pin - 3),
        PortId::A if pin <= 2 => Some(29 + pin),
        PortId::B if pin >= 11 && pin <= 15 => Some(3 + (pin - 11)),
        PortId::C if pin >= 6 && pin <= 11 => Some(8 + (pin - 6)),
        PortId::D if pin >= 9 && pin <= 15 => Some(14 + (pin - 9)),
        PortId::F if pin <= 7 => Some(21 + pin),
        _ => None,
    }
}
