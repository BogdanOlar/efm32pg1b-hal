use crate::{cmu::Clocks, gpio::Pin};
use core::{convert::Infallible, marker::PhantomData};
pub use efm32pg1b_pac::timer0::ctrl::PRESC as TimerDivider;
use efm32pg1b_pac::{
    timer0::{cc0_ctrl, cc1_ctrl, cc2_ctrl, cc3_ctrl, ctrl, RegisterBlock},
    Cmu, Timer0, Timer1,
};
use embedded_hal::{
    delay::DelayNs,
    digital::OutputPin,
    pwm::{ErrorType, SetDutyCycle},
};
use fugit::HertzU32;

pub trait TimerExt {
    type Timer;
    fn into_timer(self, clock_divider: TimerDivider) -> Self::Timer;
}

impl TimerExt for Timer0 {
    type Timer = Timer<0>;
    fn into_timer(self, clock_divider: TimerDivider) -> Self::Timer {
        Self::Timer::new(clock_divider)
    }
}

impl TimerExt for Timer1 {
    type Timer = Timer<1>;
    fn into_timer(self, clock_divider: TimerDivider) -> Self::Timer {
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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Timer<const TN: u8> {}

impl<const TN: u8> Timer<TN> {
    /// FIXME: take a (timer counter) frequency as parameter and do a best effort to set the timer prescaler and the
    ///        `top` value to get as close as possible
    fn new(clock_divider: TimerDivider) -> Self {
        let timer = timerx::<TN>();

        timer.ctrl().write(|w| {
            w.presc().variant(clock_divider);
            w.mode().variant(ctrl::MODE::Up)
        });

        // Set the resolution of the counter to MAX - 1 because if the timer is going to be split into channels and
        // any of them is used as PWM, we need to allow the PWM channel to set its compare value to TOP + 1 in order
        // to achieve 100% duty cycle
        timer.top().write(|w| unsafe { w.top().bits(u16::MAX - 1) });

        Self {}
    }

    /// Split the timer into channels which may be specialised for various uses (delay, pwm, etc.)
    pub fn into_channels(
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

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TimerChannel<const TN: u8, const CN: u8> {}

impl<const TN: u8, const CN: u8> TimerChannel<TN, CN> {
    pub fn into_pwm<PIN>(self, pin: PIN) -> TimerChannelPwm<TN, CN, PIN>
    where
        PIN: OutputPin + TimerPin<CN>,
    {
        let timer = timerx::<TN>();

        match CN {
            0 => {
                timer
                    .routeloc0()
                    .write(|w| unsafe { w.cc0loc().bits(pin.loc()) });
                timer.cc0_ctrl().write(|w| {
                    w.icedge().variant(cc0_ctrl::ICEDGE::Both);
                    w.cmoa().variant(cc0_ctrl::CMOA::Toggle);
                    w.mode().variant(cc0_ctrl::MODE::Pwm)
                });
                timer.routepen().modify(|_, w| w.cc0pen().set_bit());
            }
            1 => {
                timer
                    .routeloc0()
                    .write(|w| unsafe { w.cc1loc().bits(pin.loc()) });
                timer.cc1_ctrl().write(|w| {
                    w.icedge().variant(cc1_ctrl::ICEDGE::Both);
                    w.cmoa().variant(cc1_ctrl::CMOA::Toggle);
                    w.mode().variant(cc1_ctrl::MODE::Pwm)
                });
                timer.routepen().modify(|_, w| w.cc1pen().set_bit());
            }
            2 => {
                timer
                    .routeloc0()
                    .write(|w| unsafe { w.cc2loc().bits(pin.loc()) });
                timer.cc2_ctrl().write(|w| {
                    w.icedge().variant(cc2_ctrl::ICEDGE::Both);
                    w.cmoa().variant(cc2_ctrl::CMOA::Toggle);
                    w.mode().variant(cc2_ctrl::MODE::Pwm)
                });
                timer.routepen().modify(|_, w| w.cc2pen().set_bit());
            }
            3 => {
                timer
                    .routeloc0()
                    .write(|w| unsafe { w.cc3loc().bits(pin.loc()) });
                timer.cc3_ctrl().write(|w| {
                    w.icedge().variant(cc3_ctrl::ICEDGE::Both);
                    w.cmoa().variant(cc3_ctrl::CMOA::Toggle);
                    w.mode().variant(cc3_ctrl::MODE::Pwm)
                });
                timer.routepen().modify(|_, w| w.cc3pen().set_bit());
            }
            _ => unreachable!(),
        }

        TimerChannelPwm {
            _pwm_pin: PhantomData,
        }
    }

    pub fn into_delay(self, clocks: &Clocks) -> TimerChannelDelay<TN, CN> {
        let timer = timerx::<TN>();
        let timer_div: u8 = timer.ctrl().read().presc().variant().unwrap().into();
        let timer_freq = clocks.hf_per_clk() / (timer_div + 1) as u32;

        match CN {
            0 => timer
                .cc0_ctrl()
                .write(|w| w.mode().variant(cc0_ctrl::MODE::Outputcompare)),
            1 => timer
                .cc1_ctrl()
                .write(|w| w.mode().variant(cc1_ctrl::MODE::Outputcompare)),
            2 => timer
                .cc2_ctrl()
                .write(|w| w.mode().variant(cc2_ctrl::MODE::Outputcompare)),
            3 => timer
                .cc3_ctrl()
                .write(|w| w.mode().variant(cc3_ctrl::MODE::Outputcompare)),
            _ => unreachable!(),
        }

        TimerChannelDelay { timer_freq }
    }
}

/// Specialize the timer channel to be used for delays
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TimerChannelDelay<const TN: u8, const CN: u8> {
    timer_freq: HertzU32,
}

impl<const TN: u8, const CN: u8> DelayNs for TimerChannelDelay<TN, CN> {
    fn delay_ns(&mut self, ns: u32) {
        let microsecs = ns / 1000;

        // FIXME: converting ns to us is just a band-aid in order to avoid the delay duration being smaller than this
        //        code can handle. Worst case scenario is if the timer frequency is the same as the core frequency,
        //        in which case wanting to wait for a few nanoseconds may take longer than expected because the code
        //        below needs to calculate a Compare value which may have already elapsed by the time it's written to
        //        the compare field of the CC channel.
        //        A better accuracy may be obtained if we implement `DelayNs` for `Timer` instead of `TimerChannelDelay`
        //        since we can control when the timer starts.
        if microsecs > 0 {
            let timer = timerx::<TN>();
            let ticks_left = self.timer_freq.raw() as u64 * microsecs as u64 / 1_000_000_u64;
            let reload_max = timer.top().read().top().bits() as u32;
            let reference_count = timer.cnt().read().cnt().bits() as u32;

            let mut ticks_left = ticks_left as u32;
            let mut reload = ticks_left.min(reload_max);
            let mut compare = (reference_count + reload) % reload_max;

            while ticks_left > 0 {
                match CN {
                    0 => {
                        // clear interrupt flag
                        timer.ifc().write(|w| w.cc0().set_bit());

                        // set compare
                        timer
                            .cc0_ccv()
                            .write(|w| unsafe { w.ccv().bits(compare as u16) });

                        // enable channel interrupt
                        timer.ien().write(|w| w.cc0().set_bit());
                    }
                    1 => {
                        // clear interrupt flag
                        timer.ifc().write(|w| w.cc1().set_bit());

                        // set compare
                        timer
                            .cc1_ccv()
                            .write(|w| unsafe { w.ccv().bits(compare as u16) });

                        // enable channel interrupt
                        timer.ien().write(|w| w.cc1().set_bit());
                    }
                    2 => {
                        // clear interrupt flag
                        timer.ifc().write(|w| w.cc2().set_bit());

                        // set compare
                        timer
                            .cc2_ccv()
                            .write(|w| unsafe { w.ccv().bits(compare as u16) });

                        // enable channel interrupt
                        timer.ien().write(|w| w.cc2().set_bit());
                    }
                    3 => {
                        // clear interrupt flag
                        timer.ifc().write(|w| w.cc3().set_bit());

                        // set compare
                        timer
                            .cc3_ccv()
                            .write(|w| unsafe { w.ccv().bits(compare as u16) });

                        // enable channel interrupt
                        timer.ien().write(|w| w.cc3().set_bit());
                    }
                    _ => unreachable!(),
                }

                // calculate next loop's values _before_ waiting so that the jitter between loops is minimal
                ticks_left -= reload;
                reload = ticks_left.min(reload_max);
                compare = (reference_count + reload) % reload_max;

                match CN {
                    0 => while timer.ifl().read().cc0().bit_is_clear() {},
                    1 => while timer.ifl().read().cc1().bit_is_clear() {},
                    2 => while timer.ifl().read().cc2().bit_is_clear() {},
                    3 => while timer.ifl().read().cc3().bit_is_clear() {},
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TimerChannelPwm<const TN: u8, const CN: u8, PIN>
where
    PIN: OutputPin + TimerPin<CN>,
{
    _pwm_pin: PhantomData<PIN>,
}

impl<const TN: u8, const CN: u8, PIN> SetDutyCycle for TimerChannelPwm<TN, CN, PIN>
where
    PIN: OutputPin + TimerPin<CN>,
{
    fn max_duty_cycle(&self) -> u16 {
        // A 100% duty cycle is obtained by setting the channel Capture/Compare value to `top + 1`
        timerx::<TN>().top().read().top().bits().saturating_add(1)
    }

    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
        let timer = timerx::<TN>();

        match CN {
            0 => timer.cc0_ccvb().write(|w| unsafe { w.ccvb().bits(duty) }),
            1 => timer.cc1_ccvb().write(|w| unsafe { w.ccvb().bits(duty) }),
            2 => timer.cc2_ccvb().write(|w| unsafe { w.ccvb().bits(duty) }),
            3 => timer.cc3_ccvb().write(|w| unsafe { w.ccvb().bits(duty) }),
            _ => unreachable!(),
        }

        Ok(())
    }
}

impl<const TN: u8, const CN: u8, PIN> ErrorType for TimerChannelPwm<TN, CN, PIN>
where
    PIN: OutputPin + TimerPin<CN>,
{
    type Error = Infallible;
}

pub trait TimerPin<const CN: u8> {
    fn loc(&self) -> u8;
}

/// Implement pin location trait for each of the timer channels and their sets of 32 pins
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
