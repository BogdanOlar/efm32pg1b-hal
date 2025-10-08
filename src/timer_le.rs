use crate::gpio::pin::Pin;
use core::marker::PhantomData;
use cortex_m::asm::nop;
use efm32pg1b_pac::{
    letimer0::{ctrl::UFOA0, RegisterBlock},
    Cmu, Letimer0,
};
use embedded_hal::digital::OutputPin;

pub trait LeTimerExt {
    type Timer;
    fn into_timer(self) -> Self::Timer;
}

impl LeTimerExt for Letimer0 {
    type Timer = LeTimer;
    fn into_timer(self) -> Self::Timer {
        Self::Timer::new()
    }
}

/// Get a reference to the Low Energy Timer register block
const fn timerx() -> &'static RegisterBlock {
    unsafe { &*Letimer0::ptr() }
}

pub struct LeTimer;

impl LeTimer {
    fn new() -> Self {
        let cmu = unsafe { Cmu::steal() };

        // Enable LE Timer
        cmu.lfaclken0().modify(|_, w| w.letimer0().set_bit());

        // Sync
        while cmu.syncbusy().read().lfaclken0().bit_is_set() {
            nop()
        }

        LeTimer {}
    }

    pub fn into_ch0_pwm<PIN>(self, pin: PIN) -> LeTimerPwm<0, PIN>
    where
        PIN: OutputPin + LeTimerPin<0>,
    {
        let le_timer = timerx();

        le_timer.rep0().write(|w| unsafe { w.rep0().bits(1) });
        le_timer.comp0().write(|w| unsafe { w.comp0().bits(1000) });
        le_timer.comp1().write(|w| unsafe { w.comp1().bits(500) });
        le_timer.routepen().write(|w| w.out0pen().set_bit());
        le_timer
            .routeloc0()
            .write(|w| unsafe { w.out0loc().bits(pin.loc()) });
        le_timer.ctrl().write(|w| {
            w.comp0top().set_bit();
            w.ufoa0().variant(UFOA0::Pwm)
        });

        // start timer
        le_timer.cmd().write(|w| w.start().set_bit());

        // Sync
        while le_timer.syncbusy().read().cmd().bit_is_set() {
            nop()
        }

        LeTimerPwm {
            _pwm_pin: PhantomData,
        }
    }
}

pub struct LeTimerPwm<const CN: u8, PIN>
where
    PIN: OutputPin + LeTimerPin<CN>,
{
    _pwm_pin: PhantomData<PIN>,
}

pub trait LeTimerPin<const CN: u8> {
    fn loc(&self) -> u8;
}

/// Implement pin location trait for each of the LE timer channels and their sets of 32 pins
macro_rules! impl_le_timer_channel_loc {
    ($channel:literal, $loc:literal, $port:literal, $pin:literal) => {
        impl<ANY> LeTimerPin<$channel> for Pin<$port, $pin, ANY> {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_le_timer_channel_loc!(0, 0, 'A', 0);
impl_le_timer_channel_loc!(0, 1, 'A', 1);
impl_le_timer_channel_loc!(0, 2, 'A', 2);
impl_le_timer_channel_loc!(0, 3, 'A', 3);
impl_le_timer_channel_loc!(0, 4, 'A', 4);
impl_le_timer_channel_loc!(0, 5, 'A', 5);
impl_le_timer_channel_loc!(0, 6, 'B', 11);
impl_le_timer_channel_loc!(0, 7, 'B', 12);
impl_le_timer_channel_loc!(0, 8, 'B', 13);
impl_le_timer_channel_loc!(0, 9, 'B', 14);
impl_le_timer_channel_loc!(0, 10, 'B', 15);
impl_le_timer_channel_loc!(0, 11, 'C', 6);
impl_le_timer_channel_loc!(0, 12, 'C', 7);
impl_le_timer_channel_loc!(0, 13, 'C', 8);
impl_le_timer_channel_loc!(0, 14, 'C', 9);
impl_le_timer_channel_loc!(0, 15, 'C', 10);
impl_le_timer_channel_loc!(0, 16, 'C', 11);
impl_le_timer_channel_loc!(0, 17, 'D', 9);
impl_le_timer_channel_loc!(0, 18, 'D', 10);
impl_le_timer_channel_loc!(0, 19, 'D', 11);
impl_le_timer_channel_loc!(0, 20, 'D', 12);
impl_le_timer_channel_loc!(0, 21, 'D', 13);
impl_le_timer_channel_loc!(0, 22, 'D', 14);
impl_le_timer_channel_loc!(0, 23, 'D', 15);
impl_le_timer_channel_loc!(0, 24, 'F', 0);
impl_le_timer_channel_loc!(0, 25, 'F', 1);
impl_le_timer_channel_loc!(0, 26, 'F', 2);
impl_le_timer_channel_loc!(0, 27, 'F', 3);
impl_le_timer_channel_loc!(0, 28, 'F', 4);
impl_le_timer_channel_loc!(0, 29, 'F', 5);
impl_le_timer_channel_loc!(0, 30, 'F', 6);
impl_le_timer_channel_loc!(0, 31, 'F', 7);

impl_le_timer_channel_loc!(1, 0, 'A', 1);
impl_le_timer_channel_loc!(1, 1, 'A', 2);
impl_le_timer_channel_loc!(1, 2, 'A', 3);
impl_le_timer_channel_loc!(1, 3, 'A', 4);
impl_le_timer_channel_loc!(1, 4, 'A', 5);
impl_le_timer_channel_loc!(1, 5, 'B', 11);
impl_le_timer_channel_loc!(1, 6, 'B', 12);
impl_le_timer_channel_loc!(1, 7, 'B', 13);
impl_le_timer_channel_loc!(1, 8, 'B', 14);
impl_le_timer_channel_loc!(1, 9, 'B', 15);
impl_le_timer_channel_loc!(1, 10, 'C', 6);
impl_le_timer_channel_loc!(1, 11, 'C', 7);
impl_le_timer_channel_loc!(1, 12, 'C', 8);
impl_le_timer_channel_loc!(1, 13, 'C', 9);
impl_le_timer_channel_loc!(1, 14, 'C', 10);
impl_le_timer_channel_loc!(1, 15, 'C', 11);
impl_le_timer_channel_loc!(1, 16, 'D', 9);
impl_le_timer_channel_loc!(1, 17, 'D', 10);
impl_le_timer_channel_loc!(1, 18, 'D', 11);
impl_le_timer_channel_loc!(1, 19, 'D', 12);
impl_le_timer_channel_loc!(1, 20, 'D', 13);
impl_le_timer_channel_loc!(1, 21, 'D', 14);
impl_le_timer_channel_loc!(1, 22, 'D', 15);
impl_le_timer_channel_loc!(1, 23, 'F', 0);
impl_le_timer_channel_loc!(1, 24, 'F', 1);
impl_le_timer_channel_loc!(1, 25, 'F', 2);
impl_le_timer_channel_loc!(1, 26, 'F', 3);
impl_le_timer_channel_loc!(1, 27, 'F', 4);
impl_le_timer_channel_loc!(1, 28, 'F', 5);
impl_le_timer_channel_loc!(1, 29, 'F', 6);
impl_le_timer_channel_loc!(1, 30, 'F', 7);
impl_le_timer_channel_loc!(1, 31, 'A', 0);
