//! Gpio Port
//!
//!

#[cfg(feature = "use_debug_pins")]
use crate::gpio::debug::debug_pins_enabled;
use crate::{gpio::GpioError, Sealed};

/// Generic port type
///
/// - `P` is port name: `A` for GPIOA, `B` for GPIOB, etc.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Port<const P: char> {}

impl<const P: char> Port<P>
where
    Port<P>: Sealed,
{
    /// Construct a new `Port` with the given generic parameter `P` identifier (`A` for GPIOA, `B` for GPIOB, etc.)
    pub(crate) const fn new() -> Self {
        Self {}
    }

    /// Reset the the Port `P` registers to their reset state
    pub(crate) fn reset(&mut self) {
        let port = ports::get(self.id());
        port.dout().reset();
        port.model().reset();
        port.modeh().reset();
        port.ctrl().reset();
        port.ovt_dis().reset();
    }

    pub fn id(&self) -> PortId {
        // SAFETY: the `P` generic type parameter is guaranteed to be convertible to a `PortId`
        P.try_into().unwrap()
    }

    /// Get the Drive Strength setting of this port (not in Alternate Mode)
    pub fn drive_strength(&self) -> DriveStrength {
        ports::drive_strength(self.id())
    }

    /// Get the Alternate Drive Strength setting of this port
    pub fn drive_strength_alt(&self) -> DriveStrength {
        ports::drive_strength_alt(self.id())
    }

    /// Set the Drive Strength setting of this port (not in Alternate Mode)
    pub fn set_drive_strength(&mut self, drive_strength: DriveStrength) {
        ports::set_drive_strength(self.id(), drive_strength);
    }

    /// Set the Alternate Drive Strength setting of this port
    pub fn set_drive_strength_alt(&mut self, drive_strength: DriveStrength) {
        ports::set_drive_strength_alt(self.id(), drive_strength);
    }

    /// Get the Slew Rate setting of this port (not in Alternate Mode). Higher values represent faster slewrates.
    pub fn slew_rate(&self) -> DriveSlewRate {
        ports::slew_rate(self.id())
    }

    /// Get the Slew Rate setting of this port. Higher values represent faster slewrates.
    pub fn slew_rate_alt(&self) -> DriveSlewRate {
        ports::slew_rate_alt(self.id())
    }

    /// Set the Slew Rate setting of this port (not in Alternate Mode). Higher values represent faster slewrates
    pub fn set_slew_rate(&mut self, slew_rate: DriveSlewRate) {
        ports::set_slew_rate(self.id(), slew_rate);
    }

    /// Set the Alternate Slew Rate setting of this port. Higher values represent faster slewrates.
    pub fn set_slew_rate_alt(&mut self, slew_rate: DriveSlewRate) {
        ports::set_slew_rate_alt(self.id(), slew_rate);
    }

    /// Get the Data In Disable setting of this port (not in Alternate Mode)
    pub fn din_dis(&self) -> bool {
        ports::din_dis(self.id())
    }

    /// Get the Alternate Data In Disable setting of this port
    pub fn din_dis_alt(&self) -> bool {
        ports::din_dis_alt(self.id())
    }

    /// Set the Alternate Data In Disable setting of this port
    pub fn set_din_dis_alt(&mut self, din_dis: DataInCtrl) {
        ports::set_din_dis_alt(self.id(), din_dis);
    }
}

impl Sealed for Port<'A'> {}
impl Sealed for Port<'B'> {}
impl Sealed for Port<'C'> {}
impl Sealed for Port<'D'> {}
impl Sealed for Port<'F'> {}

/// Type safe representation of a Port ID
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PortId {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    F = 4,
}

impl TryFrom<u8> for PortId {
    type Error = GpioError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PortId::A),
            1 => Ok(PortId::B),
            2 => Ok(PortId::C),
            3 => Ok(PortId::D),
            4 => Ok(PortId::F),
            _ => Err(GpioError::InvalidPortId(value)),
        }
    }
}

impl TryFrom<char> for PortId {
    type Error = GpioError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'A' => Ok(PortId::A),
            'B' => Ok(PortId::B),
            'C' => Ok(PortId::C),
            'D' => Ok(PortId::D),
            'F' => Ok(PortId::F),
            _ => Err(GpioError::InvalidPortIdLabel(value)),
        }
    }
}

impl From<PortId> for char {
    fn from(value: PortId) -> Self {
        match value {
            PortId::A => 'A',
            PortId::B => 'B',
            PortId::C => 'C',
            PortId::D => 'D',
            PortId::F => 'F',
        }
    }
}

/// Configure GPIO peripheral registers values for individual ports
pub(crate) mod ports {
    use crate::gpio::port::{DataInCtrl, DriveSlewRate, DriveStrength, PortId};
    use crate::pac::gpio::PortA;

    /// Get the memory mapped `PortA` reference corresponding to the given `port` parameter
    ///
    /// Note: We're returning a `PortA` because all ports use the same struct (they have type aliases to this type)
    #[inline(always)]
    pub(crate) const fn get(port: PortId) -> &'static PortA {
        match port {
            PortId::A => unsafe { (*crate::pac::Gpio::ptr()).port_a() },
            PortId::B => unsafe { (*crate::pac::Gpio::ptr()).port_b() },
            PortId::C => unsafe { (*crate::pac::Gpio::ptr()).port_c() },
            PortId::D => unsafe { (*crate::pac::Gpio::ptr()).port_d() },
            PortId::F => unsafe { (*crate::pac::Gpio::ptr()).port_f() },
        }
    }

    /// Get the Drive Strength setting of this port (not in Alternate Mode)
    pub(crate) fn drive_strength(port: PortId) -> DriveStrength {
        match get(port).ctrl().read().drive_strength().bit() {
            true => DriveStrength::Weak,
            false => DriveStrength::Strong,
        }
    }

    /// Get the Alternate Drive Strength setting of this port
    pub(crate) fn drive_strength_alt(port: PortId) -> DriveStrength {
        match get(port).ctrl().read().drive_strength_alt().bit() {
            true => DriveStrength::Weak,
            false => DriveStrength::Strong,
        }
    }

    /// Set the Drive Strength setting of this port (not in Alternate Mode)
    pub(crate) fn set_drive_strength(port: PortId, drive_strength: DriveStrength) {
        get(port).ctrl().modify(|_, w| match drive_strength {
            DriveStrength::Strong => w.drive_strength().clear_bit(),
            DriveStrength::Weak => w.drive_strength().set_bit(),
        });
    }

    /// Set the Alternate Drive Strength setting of this port
    pub(crate) fn set_drive_strength_alt(port: PortId, drive_strength: DriveStrength) {
        get(port).ctrl().modify(|_, w| match drive_strength {
            DriveStrength::Strong => w.drive_strength().clear_bit(),
            DriveStrength::Weak => w.drive_strength().set_bit(),
        });
    }

    /// Get the Slew Rate setting of this port (not in Alternate Mode). Higher values represent faster slewrates.
    pub(crate) fn slew_rate(port: PortId) -> DriveSlewRate {
        // SAFETY: We've read an invalid value from a 3-bit value, which should not happen for `DriveSlewRate` which
        //         covers all 3-bit values
        get(port)
            .ctrl()
            .read()
            .slew_rate()
            .bits()
            .try_into()
            .unwrap()
    }

    /// Get the Slew Rate setting of this port. Higher values represent faster slewrates.
    pub(crate) fn slew_rate_alt(port: PortId) -> DriveSlewRate {
        // SAFETY: We've read an invalid value from a 3-bit value, which should not happen for `DriveSlewRate` which
        //         covers all 3-bit values
        get(port)
            .ctrl()
            .read()
            .slew_rate_alt()
            .bits()
            .try_into()
            .unwrap()
    }

    /// Set the Slew Rate setting of this port (not in Alternate Mode). Higher values represent faster slewrates
    pub(crate) fn set_slew_rate(port: PortId, slew_rate: DriveSlewRate) {
        get(port)
            .ctrl()
            .modify(|_, w| unsafe { w.slew_rate().bits(slew_rate.into()) });
    }

    /// Set the Alternate Slew Rate setting of this port. Higher values represent faster slewrates.
    pub(crate) fn set_slew_rate_alt(port: PortId, slew_rate: DriveSlewRate) {
        get(port)
            .ctrl()
            .modify(|_, w| unsafe { w.slew_rate_alt().bits(slew_rate.into()) });
    }

    /// Get the Data In Disable setting of this port (not in Alternate Mode)
    pub(crate) fn din_dis(port: PortId) -> bool {
        get(port).ctrl().read().din_dis().bit_is_set()
    }

    /// Get the Alternate Data In Disable setting of this port
    pub(crate) fn din_dis_alt(port: PortId) -> bool {
        get(port).ctrl().read().din_dis_alt().bit_is_set()
    }

    /// Set the Data In Disable setting of this port (not in Alternate Mode)
    pub(crate) fn set_din_dis(port: PortId, din_dis: DataInCtrl) {
        get(port).ctrl().modify(|_, w| match din_dis {
            DataInCtrl::Enabled => w.din_dis().clear_bit(),
            DataInCtrl::Disabled => w.din_dis().set_bit(),
        });
    }

    /// Set the Alternate Data In Disable setting of this port
    pub(crate) fn set_din_dis_alt(port: PortId, din_dis: DataInCtrl) {
        get(port).ctrl().modify(|_, w| match din_dis {
            DataInCtrl::Enabled => w.din_dis_alt().clear_bit(),
            DataInCtrl::Disabled => w.din_dis_alt().set_bit(),
        });
    }
}

/// Data In Disable trait used to protect the debug pins in port `F`.
///
/// Only implemented for ports `A`, `B`, `C` and `D`.
pub trait PortDataInDisable: Sealed {
    /// Set the Data In Disable setting of this port (not in Alternate Mode).
    fn set_din_dis(&mut self, din_dis: DataInCtrl);
}

impl PortDataInDisable for Port<'A'> {
    fn set_din_dis(&mut self, din_dis: DataInCtrl) {
        ports::set_din_dis(self.id(), din_dis);
    }
}

impl PortDataInDisable for Port<'B'> {
    fn set_din_dis(&mut self, din_dis: DataInCtrl) {
        ports::set_din_dis(self.id(), din_dis);
    }
}

impl PortDataInDisable for Port<'C'> {
    fn set_din_dis(&mut self, din_dis: DataInCtrl) {
        ports::set_din_dis(self.id(), din_dis);
    }
}

impl PortDataInDisable for Port<'D'> {
    fn set_din_dis(&mut self, din_dis: DataInCtrl) {
        ports::set_din_dis(self.id(), din_dis);
    }
}

/// Data In Disable trait used to protect the debug pins in port `F`.
///
/// Only implemented for port `F`, and only if the `use_debug_pins` crate feature is enabled.
pub trait PortFDataInDisable: Sealed {
    /// Set the Data In Disable setting of this port (not in Alternate Mode).
    ///
    /// The `use_debug_pins` crate feature needs to be enabled in order to have this method on port `F`.
    fn set_din_dis(&mut self, din_dis: DataInCtrl) -> Result<(), GpioError>;
}

#[cfg(feature = "use_debug_pins")]
impl PortFDataInDisable for Port<'F'> {
    fn set_din_dis(&mut self, din_dis: DataInCtrl) -> Result<(), GpioError> {
        match din_dis {
            DataInCtrl::Enabled => {
                ports::set_din_dis(self.id(), din_dis);
                Ok(())
            }
            DataInCtrl::Disabled => {
                if debug_pins_enabled() {
                    // Don't allow disabling Data In for port `F` if the debug pins (pf0-pf3) are enabled
                    Err(GpioError::DebugPinsEnabled)
                } else {
                    ports::set_din_dis(self.id(), din_dis);
                    Ok(())
                }
            }
        }
    }
}

/// Data In Control variants for `DIN_DIS` (and `ALT`) field in `GPIO_Px_CTRL` Port Control Register
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DataInCtrl {
    /// Data In is NOT disabled
    Enabled,
    /// Data In is disabled
    Disabled,
}

/// Drive current variants for `DRIVESTRENGTH` (and `ALT`) field in `GPIO_Px_CTRL` Port Control Register
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DriveStrength {
    /// Drive strength 10mA drive current
    Strong,
    /// Drive strength 1mA drive current
    Weak,
}

/// Slewrate limit for port pins. Higher values represent faster slewrates.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DriveSlewRate {
    SlewRate0,
    SlewRate1,
    SlewRate2,
    SlewRate3,
    SlewRate4,
    SlewRate5,
    SlewRate6,
    SlewRate7,
}

impl From<DriveSlewRate> for u8 {
    fn from(slew_rate: DriveSlewRate) -> Self {
        match slew_rate {
            DriveSlewRate::SlewRate0 => 0,
            DriveSlewRate::SlewRate1 => 1,
            DriveSlewRate::SlewRate2 => 2,
            DriveSlewRate::SlewRate3 => 3,
            DriveSlewRate::SlewRate4 => 4,
            DriveSlewRate::SlewRate5 => 5,
            DriveSlewRate::SlewRate6 => 6,
            DriveSlewRate::SlewRate7 => 7,
        }
    }
}

impl TryFrom<u8> for DriveSlewRate {
    type Error = GpioError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DriveSlewRate::SlewRate0),
            1 => Ok(DriveSlewRate::SlewRate1),
            2 => Ok(DriveSlewRate::SlewRate2),
            3 => Ok(DriveSlewRate::SlewRate3),
            4 => Ok(DriveSlewRate::SlewRate4),
            5 => Ok(DriveSlewRate::SlewRate5),
            6 => Ok(DriveSlewRate::SlewRate6),
            7 => Ok(DriveSlewRate::SlewRate7),
            x => Err(GpioError::InvalidSlewRate(x)),
        }
    }
}
