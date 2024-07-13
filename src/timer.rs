use crate::{cmu::Clocks, gpio::Pin};
use core::marker::PhantomData;
pub use efm32pg1b_pac::timer0::ctrl::PRESC as TimerDivider;
use efm32pg1b_pac::{
    timer0::{ctrl::MODE, RegisterBlock},
    Cmu, Timer0, Timer1,
};
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use fugit::HertzU32;

pub trait TimerExt {
    type Timer;
    fn new(self, clock_divider: TimerDivider) -> Self::Timer;
}

impl TimerExt for Timer0 {
    type Timer = Timer<0>;
    fn new(self, clock_divider: TimerDivider) -> Self::Timer {
        Self::Timer::new(clock_divider)
    }
}

impl TimerExt for Timer1 {
    type Timer = Timer<1>;
    fn new(self, clock_divider: TimerDivider) -> Self::Timer {
        Self::Timer::new(clock_divider)
    }
}

/// Get a reference to one of the two timers register block, specified by `TN` (either `Timer0`, or `Timer1`)
const fn timerx<const TN: u8>() -> &'static RegisterBlock {
    match TN {
        0 => unsafe { &*Timer0::ptr() },
        1 => unsafe { &*Timer1::ptr() },
        _ => unreachable!(),
    }
}

pub struct Timer<const TN: u8> {}

impl<const TN: u8> Timer<TN> {
    fn new(clock_divider: TimerDivider) -> Self {
        let timer = timerx::<TN>();

        timer.ctrl().write(|w| {
            w.presc().variant(clock_divider);
            w.mode().variant(MODE::Up)
        });

        Self {}
    }

    pub fn split(
        self,
    ) -> (
        TimerChannel<TN, 0>,
        TimerChannel<TN, 1>,
        TimerChannel<TN, 2>,
        TimerChannel<TN, 3>,
    ) {
        // enable Timer<TN> peripheral clock
        match TN {
            0 => unsafe {
                Cmu::steal()
                    .hfperclken0()
                    .modify(|_, w| w.timer0().set_bit());
            },
            1 => unsafe {
                Cmu::steal()
                    .hfperclken0()
                    .modify(|_, w| w.timer1().set_bit());
            },
            _ => unreachable!(),
        }

        // Enable timer
        timerx::<TN>().cmd().write(|w| w.start().set_bit());

        // Split the peripheral into its channels
        (
            TimerChannel {},
            TimerChannel {},
            TimerChannel {},
            TimerChannel {},
        )
    }
}

pub struct TimerChannel<const TN: u8, const CN: u8> {}

impl<const TN: u8, const CN: u8> TimerChannel<TN, CN> {
    pub fn into_pwm<PIN>(self, pin: PIN) -> TimerChannelPwm<TN, CN, PIN>
    where
        PIN: OutputPin + TimerPin<CN>,
    {
        todo!()
    }

    pub fn into_delay(self, clocks: &Clocks) -> TimerChannelDelay<TN, CN> {
        let timer = timerx::<TN>();
        let timer_div: u8 = timer.ctrl().read().presc().variant().unwrap().into();
        let timer_freq = clocks.hf_per_clk / (timer_div + 1) as u32;
        TimerChannelDelay { timer_freq }
    }
}

pub struct TimerChannelPwm<const TN: u8, const CN: u8, PIN>
where
    PIN: OutputPin + TimerPin<CN>,
{
    _pwm_pin: PhantomData<PIN>,
}

/// Specialize the timer channel to be used for delays
pub struct TimerChannelDelay<const TN: u8, const CN: u8> {
    timer_freq: HertzU32,
}

impl<const TN: u8, const CN: u8> DelayNs for TimerChannelDelay<TN, CN> {
    fn delay_ns(&mut self, ns: u32) {
        todo!()
    }
}

pub trait TimerPin<const CN: u8> {
    fn loc(&self) -> u8;
}

/// Implement pin location trait for each of the timer channels
macro_rules! impl_timer_channel_loc {
    ($channel:literal, $loc:literal, $port:literal, $pin:literal) => {
        impl<ANY> TimerPin<$channel> for Pin<$port, $pin, ANY> {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_timer_channel_loc!(0, 0, 'A', 0);
impl_timer_channel_loc!(0, 1, 'A', 1);
impl_timer_channel_loc!(0, 2, 'A', 2);
impl_timer_channel_loc!(0, 3, 'A', 3);
impl_timer_channel_loc!(0, 4, 'A', 4);
impl_timer_channel_loc!(0, 5, 'A', 5);
impl_timer_channel_loc!(0, 6, 'B', 11);
impl_timer_channel_loc!(0, 7, 'B', 12);
impl_timer_channel_loc!(0, 8, 'B', 13);
impl_timer_channel_loc!(0, 9, 'B', 14);
impl_timer_channel_loc!(0, 10, 'B', 15);
impl_timer_channel_loc!(0, 11, 'C', 6);
impl_timer_channel_loc!(0, 12, 'C', 7);
impl_timer_channel_loc!(0, 13, 'C', 8);
impl_timer_channel_loc!(0, 14, 'C', 9);
impl_timer_channel_loc!(0, 15, 'C', 10);
impl_timer_channel_loc!(0, 16, 'C', 11);
impl_timer_channel_loc!(0, 17, 'D', 9);
impl_timer_channel_loc!(0, 18, 'D', 10);
impl_timer_channel_loc!(0, 19, 'D', 11);
impl_timer_channel_loc!(0, 20, 'D', 12);
impl_timer_channel_loc!(0, 21, 'D', 13);
impl_timer_channel_loc!(0, 22, 'D', 14);
impl_timer_channel_loc!(0, 23, 'D', 15);
impl_timer_channel_loc!(0, 24, 'F', 0);
impl_timer_channel_loc!(0, 25, 'F', 1);
impl_timer_channel_loc!(0, 26, 'F', 2);
impl_timer_channel_loc!(0, 27, 'F', 3);
impl_timer_channel_loc!(0, 28, 'F', 4);
impl_timer_channel_loc!(0, 29, 'F', 5);
impl_timer_channel_loc!(0, 30, 'F', 6);
impl_timer_channel_loc!(0, 31, 'F', 7);

impl_timer_channel_loc!(1, 0, 'A', 1);
impl_timer_channel_loc!(1, 1, 'A', 2);
impl_timer_channel_loc!(1, 2, 'A', 3);
impl_timer_channel_loc!(1, 3, 'A', 4);
impl_timer_channel_loc!(1, 4, 'A', 5);
impl_timer_channel_loc!(1, 5, 'B', 11);
impl_timer_channel_loc!(1, 6, 'B', 12);
impl_timer_channel_loc!(1, 7, 'B', 13);
impl_timer_channel_loc!(1, 8, 'B', 14);
impl_timer_channel_loc!(1, 9, 'B', 15);
impl_timer_channel_loc!(1, 10, 'C', 6);
impl_timer_channel_loc!(1, 11, 'C', 7);
impl_timer_channel_loc!(1, 12, 'C', 8);
impl_timer_channel_loc!(1, 13, 'C', 9);
impl_timer_channel_loc!(1, 14, 'C', 10);
impl_timer_channel_loc!(1, 15, 'C', 11);
impl_timer_channel_loc!(1, 16, 'D', 9);
impl_timer_channel_loc!(1, 17, 'D', 10);
impl_timer_channel_loc!(1, 18, 'D', 11);
impl_timer_channel_loc!(1, 19, 'D', 12);
impl_timer_channel_loc!(1, 20, 'D', 13);
impl_timer_channel_loc!(1, 21, 'D', 14);
impl_timer_channel_loc!(1, 22, 'D', 15);
impl_timer_channel_loc!(1, 23, 'F', 0);
impl_timer_channel_loc!(1, 24, 'F', 1);
impl_timer_channel_loc!(1, 25, 'F', 2);
impl_timer_channel_loc!(1, 26, 'F', 3);
impl_timer_channel_loc!(1, 27, 'F', 4);
impl_timer_channel_loc!(1, 28, 'F', 5);
impl_timer_channel_loc!(1, 29, 'F', 6);
impl_timer_channel_loc!(1, 30, 'F', 7);
impl_timer_channel_loc!(1, 31, 'A', 0);

impl_timer_channel_loc!(2, 0, 'A', 2);
impl_timer_channel_loc!(2, 1, 'A', 3);
impl_timer_channel_loc!(2, 2, 'A', 4);
impl_timer_channel_loc!(2, 3, 'A', 5);
impl_timer_channel_loc!(2, 4, 'B', 11);
impl_timer_channel_loc!(2, 5, 'B', 12);
impl_timer_channel_loc!(2, 6, 'B', 13);
impl_timer_channel_loc!(2, 7, 'B', 14);
impl_timer_channel_loc!(2, 8, 'B', 15);
impl_timer_channel_loc!(2, 9, 'C', 6);
impl_timer_channel_loc!(2, 10, 'C', 7);
impl_timer_channel_loc!(2, 11, 'C', 8);
impl_timer_channel_loc!(2, 12, 'C', 9);
impl_timer_channel_loc!(2, 13, 'C', 10);
impl_timer_channel_loc!(2, 14, 'C', 11);
impl_timer_channel_loc!(2, 15, 'D', 9);
impl_timer_channel_loc!(2, 16, 'D', 10);
impl_timer_channel_loc!(2, 17, 'D', 11);
impl_timer_channel_loc!(2, 18, 'D', 12);
impl_timer_channel_loc!(2, 19, 'D', 13);
impl_timer_channel_loc!(2, 20, 'D', 14);
impl_timer_channel_loc!(2, 21, 'D', 15);
impl_timer_channel_loc!(2, 22, 'F', 0);
impl_timer_channel_loc!(2, 23, 'F', 1);
impl_timer_channel_loc!(2, 24, 'F', 2);
impl_timer_channel_loc!(2, 25, 'F', 3);
impl_timer_channel_loc!(2, 26, 'F', 4);
impl_timer_channel_loc!(2, 27, 'F', 5);
impl_timer_channel_loc!(2, 28, 'F', 6);
impl_timer_channel_loc!(2, 29, 'F', 7);
impl_timer_channel_loc!(2, 30, 'A', 0);
impl_timer_channel_loc!(2, 31, 'A', 1);

impl_timer_channel_loc!(3, 0, 'A', 3);
impl_timer_channel_loc!(3, 1, 'A', 4);
impl_timer_channel_loc!(3, 2, 'A', 5);
impl_timer_channel_loc!(3, 3, 'B', 11);
impl_timer_channel_loc!(3, 4, 'B', 12);
impl_timer_channel_loc!(3, 5, 'B', 13);
impl_timer_channel_loc!(3, 6, 'B', 14);
impl_timer_channel_loc!(3, 7, 'B', 15);
impl_timer_channel_loc!(3, 8, 'C', 6);
impl_timer_channel_loc!(3, 9, 'C', 7);
impl_timer_channel_loc!(3, 10, 'C', 8);
impl_timer_channel_loc!(3, 11, 'C', 9);
impl_timer_channel_loc!(3, 12, 'C', 10);
impl_timer_channel_loc!(3, 13, 'C', 11);
impl_timer_channel_loc!(3, 14, 'D', 9);
impl_timer_channel_loc!(3, 15, 'D', 10);
impl_timer_channel_loc!(3, 16, 'D', 11);
impl_timer_channel_loc!(3, 17, 'D', 12);
impl_timer_channel_loc!(3, 18, 'D', 13);
impl_timer_channel_loc!(3, 19, 'D', 14);
impl_timer_channel_loc!(3, 20, 'D', 15);
impl_timer_channel_loc!(3, 21, 'F', 0);
impl_timer_channel_loc!(3, 22, 'F', 1);
impl_timer_channel_loc!(3, 23, 'F', 2);
impl_timer_channel_loc!(3, 24, 'F', 3);
impl_timer_channel_loc!(3, 25, 'F', 4);
impl_timer_channel_loc!(3, 26, 'F', 5);
impl_timer_channel_loc!(3, 27, 'F', 6);
impl_timer_channel_loc!(3, 28, 'F', 7);
impl_timer_channel_loc!(3, 29, 'A', 0);
impl_timer_channel_loc!(3, 30, 'A', 1);
impl_timer_channel_loc!(3, 31, 'A', 2);
