//! GPIO External Interrupts
//!

use crate::gpio::{pin::PinInfo, GpioError};

/// Controller for External Interrupt `N`
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExtiCtrl<const N: u8> {
    /// Ensure only this crate can instantiate `ExtiCtrl` (see [`ExtiCtrl::new`])
    _p: (),
}

impl<const N: u8> ExtiCtrl<N> {
    /// Create the controller for External Interrupt `N`
    pub(crate) fn new() -> Self {
        Self { _p: () }
    }

    /// Get the External Interrupt ID from this controller
    pub fn id(&self) -> ExtiId {
        ExtiId::from_u8_unchecked(N)
    }

    /// Sellect the interrupt edge for this Exti
    pub fn edge_select(&mut self, edge: ExtiEdge) {
        mmio::exti_edge_select(self.id(), edge);
    }

    /// Enable interrupts for this Exti
    pub fn enable(&mut self) {
        mmio::exti_enable(self.id());
    }
}

/// This trait is implemented for `Pin`s in order to constrain which pins can be bound to which External Interrupt based
/// on the Exti group number `GN`.
///
/// See also [`ExtiBind`], [`mmio::exti_bind`]
pub trait ExtiGroup<const GN: u8> {}

/// Bind an External Interrupt to a `Pin` which satisfied the [`ExtiGroup`] constraint
pub trait ExtiBind<const GN: u8> {
    /// Bind an External Interrupt to a Pin which _can_ be bound to the Exti
    fn bind<T: PinInfo + ExtiGroup<GN>>(self, pin: &T) -> Self;
}

/// Implement [`ExtiBind`] for an [`ExtiCtrl<N>`] concrete type
macro_rules! impl_exti_bind {
    ($group_number:literal, $exti_number: literal) => {
        impl ExtiBind<$group_number> for ExtiCtrl<$exti_number> {
            fn bind<T: PinInfo + ExtiGroup<$group_number>>(self, pin: &T) -> Self {
                mmio::exti_bind_unchecked(self.id(), pin.port(), pin.pin());
                self
            }
        }
    };
}

impl_exti_bind!(0, 0);
impl_exti_bind!(0, 1);
impl_exti_bind!(0, 2);
impl_exti_bind!(0, 3);
impl_exti_bind!(1, 4);
impl_exti_bind!(1, 5);
impl_exti_bind!(1, 6);
impl_exti_bind!(1, 7);
impl_exti_bind!(2, 8);
impl_exti_bind!(2, 9);
impl_exti_bind!(2, 10);
impl_exti_bind!(2, 11);
impl_exti_bind!(3, 12);
impl_exti_bind!(3, 13);
impl_exti_bind!(3, 14);
impl_exti_bind!(3, 15);

/// External Interrupt ID
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ExtiId {
    /// External Interrupt 0
    Exti0,
    /// External Interrupt 1
    Exti1,
    /// External Interrupt 2
    Exti2,
    /// External Interrupt 3
    Exti3,
    /// External Interrupt 4
    Exti4,
    /// External Interrupt 5
    Exti5,
    /// External Interrupt 6
    Exti6,
    /// External Interrupt 7
    Exti7,
    /// External Interrupt 8
    Exti8,
    /// External Interrupt 9
    Exti9,
    /// External Interrupt 10
    Exti10,
    /// External Interrupt 11
    Exti11,
    /// External Interrupt 12
    Exti12,
    /// External Interrupt 13
    Exti13,
    /// External Interrupt 14
    Exti14,
    /// External Interrupt 15
    Exti15,
}

impl ExtiId {
    pub(crate) const fn from_u8_unchecked(e: u8) -> Self {
        match e & 0b1111 {
            0 => Self::Exti0,
            1 => Self::Exti1,
            2 => Self::Exti2,
            3 => Self::Exti3,
            4 => Self::Exti4,
            5 => Self::Exti5,
            6 => Self::Exti6,
            7 => Self::Exti7,
            8 => Self::Exti8,
            9 => Self::Exti9,
            10 => Self::Exti10,
            11 => Self::Exti11,
            12 => Self::Exti12,
            13 => Self::Exti13,
            14 => Self::Exti14,
            15 => Self::Exti15,
            _ => unreachable!(),
        }
    }
}

impl TryFrom<u8> for ExtiId {
    type Error = GpioError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Exti0),
            1 => Ok(Self::Exti1),
            2 => Ok(Self::Exti2),
            3 => Ok(Self::Exti3),
            4 => Ok(Self::Exti4),
            5 => Ok(Self::Exti5),
            6 => Ok(Self::Exti6),
            7 => Ok(Self::Exti7),
            8 => Ok(Self::Exti8),
            9 => Ok(Self::Exti9),
            10 => Ok(Self::Exti10),
            11 => Ok(Self::Exti11),
            12 => Ok(Self::Exti12),
            13 => Ok(Self::Exti13),
            14 => Ok(Self::Exti14),
            15 => Ok(Self::Exti15),
            _ => Err(GpioError::InvalidExiValue(value)),
        }
    }
}

/// External Interrupt Edge
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ExtiEdge {
    /// Rising edge
    Rising,
    /// Falling edge
    Falling,
    /// Both Rising and Falling edges
    Both,
}

/// Access functions for external interrupts Memory Mapped IO
/// FIXME: make this and all `pub fn` below `pub(crate)`
pub mod mmio {

    use crate::{
        gpio::{
            exti::{ExtiEdge, ExtiId},
            pin::PinId,
            port::PortId,
            GpioError,
        },
        pac::Gpio,
    };

    const SEL_GROUP_SIZE: u8 = 4;
    /// Selection groups per register (either High or Low versions of `EXTIPSEL` and `EXTIPINSEL` registers)
    const SEL_GROUPS: u8 = u32::BITS as u8 / SEL_GROUP_SIZE;
    const SEL_PORT_BIT_MASK: u32 = 0x0F;
    const SEL_PIN_BIT_MASK: u32 = 0b11;

    /// Bind an External Interrupt to a particular port and pin.
    ///
    /// While any Exti can be assigned to any port, only some of the pins of that port can be bound to an Exti.
    ///
    /// Also, external interrupts with an even number will trigger the `GPIO_EVEN` ISR, and odd will trigger `GPIO_ODD`
    ///
    /// | Exti | Port |      Pins      | NVIC Interrupt |
    /// |------|------|----------------|----------------|
    /// |   0  |  any |  0,  1,  2,  3 |  `GPIO_EVEN`   |
    /// |   1  |  any |  0,  1,  2,  3 |  `GPIO_ODD`    |
    /// |   2  |  any |  0,  1,  2,  3 |  `GPIO_EVEN`   |
    /// |   3  |  any |  0,  1,  2,  3 |  `GPIO_ODD`    |
    /// |   4  |  any |  4,  5,  6,  7 |  `GPIO_EVEN`   |
    /// |   5  |  any |  4,  5,  6,  7 |  `GPIO_ODD`    |
    /// |   6  |  any |  4,  5,  6,  7 |  `GPIO_EVEN`   |
    /// |   7  |  any |  4,  5,  6,  7 |  `GPIO_ODD`    |
    /// |   8  |  any |  8,  9, 10, 11 |  `GPIO_EVEN`   |
    /// |   9  |  any |  8,  9, 10, 11 |  `GPIO_ODD`    |
    /// |  10  |  any |  8,  9, 10, 11 |  `GPIO_EVEN`   |
    /// |  11  |  any |  8,  9, 10, 11 |  `GPIO_ODD`    |
    /// |  12  |  any | 12, 13, 14, 15 |  `GPIO_EVEN`   |
    /// |  13  |  any | 12, 13, 14, 15 |  `GPIO_ODD`    |
    /// |  14  |  any | 12, 13, 14, 15 |  `GPIO_EVEN`   |
    /// |  15  |  any | 12, 13, 14, 15 |  `GPIO_ODD`    |
    ///
    pub fn exti_bind(exti: ExtiId, port: PortId, pin: PinId) -> Result<(), GpioError> {
        if exti_is_bind_valid(exti, pin) {
            exti_bind_unchecked(exti, port, pin);

            Ok(())
        } else {
            Err(GpioError::InvalidExiBind { exti, port, pin })
        }
    }

    pub(crate) fn exti_bind_unchecked(exti: ExtiId, port: PortId, pin: PinId) {
        let gpio = gpio();
        let offset = (exti as u8 % SEL_GROUPS) * SEL_GROUP_SIZE;
        let port_reg_val = (port as u32) << offset;
        let port_mask = SEL_PORT_BIT_MASK << offset;
        let pin_reg_val = ((pin as u32) % SEL_GROUP_SIZE as u32) << offset;
        let pin_mask = SEL_PIN_BIT_MASK << offset;

        if exti_is_low_reg(exti) {
            gpio.extipsell()
                .modify(|r, w| unsafe { w.bits((r.bits() & !port_mask) | port_reg_val) });
            gpio.extipinsell()
                .modify(|r, w| unsafe { w.bits((r.bits() & !pin_mask) | pin_reg_val) });
        } else {
            gpio.extipselh()
                .modify(|r, w| unsafe { w.bits((r.bits() & !port_mask) | port_reg_val) });
            gpio.extipinselh()
                .modify(|r, w| unsafe { w.bits((r.bits() & !pin_mask) | pin_reg_val) });
        }
    }

    /// Get the pin binding of the given external interrupt ID
    pub fn exti_bind_get(exti: ExtiId) -> (PortId, PinId) {
        let gpio = gpio();

        let pin_base_id = (exti as u8 / SEL_GROUP_SIZE) * SEL_GROUP_SIZE;
        let offset = (exti as u8 % SEL_GROUPS) * SEL_GROUP_SIZE;
        let port_mask = SEL_PORT_BIT_MASK << offset;
        let pin_mask = SEL_PIN_BIT_MASK << offset;

        let (port_reg_val, pin_reg_val) = match exti_is_low_reg(exti) {
            true => (
                gpio.extipsell().read().bits(),
                gpio.extipinsell().read().bits(),
            ),
            false => (
                gpio.extipselh().read().bits(),
                gpio.extipinselh().read().bits(),
            ),
        };
        let port = ((port_reg_val & port_mask) >> offset) as u8;
        let pin = ((pin_reg_val & pin_mask) >> offset) as u8 + pin_base_id;

        (
            PortId::from_u8_unchecked(port),
            PinId::from_u8_unchecked(pin),
        )
    }

    /// Enable given external interrupt
    pub fn exti_enable(exti: ExtiId) {
        gpio()
            .ien()
            .modify(|r, w| unsafe { w.ext().bits(r.ext().bits() | 1 << exti as u8) });
    }

    /// Checl if the interrupt flag is raised for the given external interrupt
    pub fn exti_get(exti: ExtiId) -> bool {
        (gpio().if_().read().ext().bits() & (1 << (exti as u8))) != 0
    }

    /// Iterator over all raised EVEN external interrupt flags
    pub fn exti_flags_even() -> impl Iterator<Item = ExtiId> {
        let exti_cached_flags = gpio().if_().read().ext().bits();

        (ExtiId::Exti0 as u8..=ExtiId::Exti14 as u8)
            .step_by(2)
            .filter(move |i| ((1 << *i) & exti_cached_flags) != 0)
            .map(ExtiId::from_u8_unchecked)
    }

    /// Iterator over all raised ODD external interrupt flags
    pub fn exti_flags_odd() -> impl Iterator<Item = ExtiId> {
        let exti_cached_flags = gpio().if_().read().ext().bits();

        (ExtiId::Exti1 as u8..=ExtiId::Exti15 as u8)
            .step_by(2)
            .filter(move |i| ((1 << *i) & exti_cached_flags) != 0)
            .map(ExtiId::from_u8_unchecked)
    }

    /// Clear external interrupt flag
    pub fn exti_clear(exti: ExtiId) {
        gpio()
            .ifc()
            .write(|w| unsafe { w.ext().bits(1 << (exti as u8)) });
    }

    /// Select the edge which triggers the external interrupt
    pub fn exti_edge_select(exti: ExtiId, edge: ExtiEdge) {
        let gpio = gpio();
        let exti_mask = 1 << exti as u8;

        match edge {
            ExtiEdge::Rising => {
                gpio.extirise()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
                gpio.extifall()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !exti_mask) });
            }
            ExtiEdge::Falling => {
                gpio.extirise()
                    .modify(|r, w| unsafe { w.bits(r.bits() & !exti_mask) });
                gpio.extifall()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
            }
            ExtiEdge::Both => {
                gpio.extirise()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
                gpio.extifall()
                    .modify(|r, w| unsafe { w.bits(r.bits() | exti_mask) });
            }
        };
    }

    /// Get the edge which triggers the external interrupt
    pub fn exti_edge_get(exti: ExtiId) -> Option<ExtiEdge> {
        let gpio = gpio();
        let exti_mask = 1 << exti as u8;

        let rising = (gpio.extirise().read().bits() & exti_mask) != 0;
        let falling = (gpio.extifall().read().bits() & exti_mask) != 0;

        if rising && falling {
            Some(ExtiEdge::Both)
        } else if rising {
            Some(ExtiEdge::Rising)
        } else if falling {
            Some(ExtiEdge::Falling)
        } else {
            None
        }
    }

    /// Clear the edge which triggers the external interrupt
    pub fn exti_edge_clear(exti: ExtiId, edge: ExtiEdge) {
        let gpio = gpio();
        let exti_mask = !(1 << exti as u8);

        if edge == ExtiEdge::Rising || edge == ExtiEdge::Both {
            gpio.extirise()
                .modify(|r, w| unsafe { w.bits(r.bits() & exti_mask) });
        }

        if edge == ExtiEdge::Falling || edge == ExtiEdge::Both {
            gpio.extifall()
                .modify(|r, w| unsafe { w.bits(r.bits() & exti_mask) });
        }
    }

    /// Enable energy mode 4 wake up for the given external interrupt
    pub fn exti_enable_em4wu(exti: ExtiId) {
        gpio()
            .ien()
            .modify(|_, w| unsafe { w.em4wu().bits(1 << exti as u8) });
    }

    const fn exti_is_bind_valid(exti: ExtiId, pin: PinId) -> bool {
        let exti_group = exti as u8 / SEL_GROUP_SIZE;
        let pin_group = pin as u8 / SEL_GROUP_SIZE;
        exti_group == pin_group
    }

    const fn exti_is_low_reg(exti: ExtiId) -> bool {
        (exti as u8) < (ExtiId::Exti8 as u8)
    }

    #[inline(always)]
    fn gpio() -> Gpio {
        unsafe { crate::pac::Gpio::steal() }
    }
}
