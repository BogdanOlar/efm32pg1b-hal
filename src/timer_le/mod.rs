//! Low Energy Timer
//!

#[cfg(all(
    feature = "efemb",
    any(
        feature = "efemb-timdrv-letim0-hz-32_768",
        feature = "efemb-timdrv-letim0-hz-1_000"
    )
))]
pub mod efemb;

use crate::{
    gpio::pin::Pin,
    pac::{letimer::vals::{Out0loc, Ufoa0}, CMU},
};
use core::marker::PhantomData;
use cortex_m::asm::nop;
use embedded_hal::digital::OutputPin;

/// Extension trait for Letimer PAC peripheral
pub trait LeTimerExt {
    /// Timer type
    type Timer;
    /// Convert to HAL timer
    fn into_timer(self) -> Self::Timer;
}

impl LeTimerExt for efm32pg1b_pac::letimer::Letimer {
    type Timer = LeTimer;
    fn into_timer(self) -> Self::Timer {
        Self::Timer::new()
    }
}

/// Low Energy timer
pub struct LeTimer;

impl LeTimer {
    fn new() -> Self {
        
        // Enable LE Timer
        CMU.lfaclken0().modify(|w| w.set_letimer0(true));

        // Sync
        while CMU.syncbusy().read().lfaclken0() == true {
            nop()
        }

        LeTimer {}
    }

    /// Convert timer to PWM
    pub fn into_ch0_pwm<PIN>(self, pin: PIN) -> LeTimerPwm<0, PIN>
    where
        PIN: OutputPin + LeTimerPin<0>,
    {
        let le_timer = mmio::timer_le();

        le_timer.rep0().write(|w| w.set_rep0(1));
        le_timer.comp0().write(|w| w.set_comp0(1000));
        le_timer.comp1().write(|w| w.set_comp1(500));
        le_timer.routepen().write(|w| w.set_out0pen(true));
        le_timer
            .routeloc0()
            .write(|w| w.set_out0loc(Out0loc::from_bits(pin.loc())));
        le_timer.ctrl().write(|w| {
            w.set_comp0top(true);
            w.set_ufoa0(Ufoa0::Pwm)
        });

        // start timer
        le_timer.cmd().write(|w| w.set_start(true));

        // Sync
        while le_timer.syncbusy().read().cmd() == true {
            nop()
        }

        LeTimerPwm {
            _pwm_pin: PhantomData,
        }
    }
}

mod mmio {
    use cortex_m::asm::nop;
    use efm32pg1b_pac::{letimer::Letimer as Timer, LETIMER};

    /// Reset the timer peripheral
    ///
    /// NOTE: this assumes that the peripheral is stopped and ready to be configured
    pub(crate) fn reset() {
        let p = timer_le();
        p.ctrl().write_value(Default::default());
        p.ien().write_value(Default::default());
        p.ifc().write(|w| {
            w.set_comp0(true);
            w.set_comp1(true);
            w.set_uf(true);
            w.set_rep0(true);
            w.set_rep1(true);
        });
    }

    /// Is the timer currently running
    pub(crate) fn running() -> bool {
        timer_le().status().read().running() == true
    }

    /// Low Energy Timer Interrupt Flags
    #[repr(C)]
    pub enum InterruptFlag {
        /// Comparator 0 interrupt flag
        Comp0,
        /// Comparator 1 interrupt flag
        Comp1,
        /// Underflow interrupt flag
        Underflow,
        /// Repeat 0 interrupt flag
        Rep0,
        /// Repeat 1 interrupt flag
        Rep1,
    }

    /// Get the (logical) counter value.
    ///
    /// NOTE: This is a count _down_ timer, so actual register value is `u16::MAX - cnt`
    #[inline(always)]
    pub(crate) fn counter_get() -> u16 {
        u16::MAX - timer_le().cnt().read().cnt()
    }

    /// Set the (logical) counter value.
    ///
    /// NOTE: This is a count _down_ timer, so actual register value will be set to `u16::MAX - cnt`
    pub(crate) fn counter_set(cnt: u16) {
        timer_le()
            .cnt()
            .write(|w| w.set_cnt(u16::MAX - cnt));
    }

    /// Set the (logical) comparator 0 value
    pub(crate) fn comp0_set(cnt: u16) {
        timer_le()
            .comp0()
            .write(|w| w.set_comp0(u16::MAX - cnt));
    }

    /// Set the (logical) comparator 0 value
    pub(crate) fn comp1_set(cnt: u16) {
        timer_le()
            .comp1()
            .write(|w| w.set_comp1(u16::MAX - cnt));
    }

    /// Get the state of the given interrupt flag
    pub(crate) fn if_get(flag: InterruptFlag) -> bool {
        let r = timer_le().if_().read();
        match flag {
            InterruptFlag::Comp0 => r.comp0(),
            InterruptFlag::Comp1 => r.comp1(),
            InterruptFlag::Underflow => r.uf(),
            InterruptFlag::Rep0 => r.rep0(),
            InterruptFlag::Rep1 => r.rep1(),
        }
    }

    /// Set the given interrupt flag
    pub(crate) fn if_set(flag: InterruptFlag) {
        timer_le().ifs().write(|w| match flag {
            InterruptFlag::Comp0 => w.set_comp0(true),
            InterruptFlag::Comp1 => w.set_comp1(true),
            InterruptFlag::Underflow => w.set_uf(true),
            InterruptFlag::Rep0 => w.set_rep0(true),
            InterruptFlag::Rep1 => w.set_rep1(true),
        });
    }

    /// Clear the given interrupt flag
    pub(crate) fn if_clear(flag: InterruptFlag) {
        timer_le().ifc().write(|w| match flag {
            InterruptFlag::Comp0 => w.set_comp0(true),
            InterruptFlag::Comp1 => w.set_comp1(true),
            InterruptFlag::Underflow => w.set_uf(true),
            InterruptFlag::Rep0 => w.set_rep0(true),
            InterruptFlag::Rep1 => w.set_rep1(true),
        });
    }

    /// Enable the given interrupt flag
    pub(crate) fn ienable(flag: InterruptFlag) {
        timer_le().ien().modify(|w| match flag {
            InterruptFlag::Comp0 => w.set_comp0(true),
            InterruptFlag::Comp1 => w.set_comp1(true),
            InterruptFlag::Underflow => w.set_uf(true),
            InterruptFlag::Rep0 => w.set_rep0(true),
            InterruptFlag::Rep1 => w.set_rep1(true),
        });
    }

    pub(crate) fn idisable(flag: InterruptFlag) {
        timer_le().ien().modify(|w| match flag {
            InterruptFlag::Comp0 => w.set_comp0(false),
            InterruptFlag::Comp1 => w.set_comp1(false),
            InterruptFlag::Underflow => w.set_uf(false),
            InterruptFlag::Rep0 => w.set_rep0(false),
            InterruptFlag::Rep1 => w.set_rep1(false),
        });
    }

    /// Timer commands
    #[repr(C)]
    pub enum Command {
        /// Start the timer
        Start,
        /// Stop the timer
        Stop,
        /// Clear the timer counter register to `0`
        Clear,
        /// Drive toggle output 0 to its idle value
        ClearToggleOutput0,
        /// Drive toggle output 1 to its idle value
        ClearToggleOutput1,
    }

    /// Execute a timer command and wait until the command is applied
    pub(crate) fn cmd(command: Command) {
        cmds(&[command]);
    }

    /// Execute multiple timer command and wait until the commands are applied
    pub(crate) fn cmds(commands: &[Command]) {
        let p = timer_le();
        p.cmd().write(|w| {
            for cmd in commands {
                match cmd {
                    Command::Start => {
                        w.set_start(true);
                    }
                    Command::Stop => {
                        w.set_stop(true);
                    }
                    Command::Clear => {
                        w.set_clear(true);
                    }
                    Command::ClearToggleOutput0 => {
                        w.set_cto0(true);
                    }
                    Command::ClearToggleOutput1 => {
                        w.set_cto1(true);
                    }
                }
            }
        });

        // Block until the timer commands have been applied
        while p.syncbusy().read().cmd() == true {
            nop();
        }
    }

    /// Get a reference to the Low Energy Timer register block
    pub(crate) const fn timer_le() -> Timer {
        LETIMER
    }
}

/// Low Energy Timer PWM
pub struct LeTimerPwm<const CN: u8, PIN>
where
    PIN: OutputPin + LeTimerPin<CN>,
{
    _pwm_pin: PhantomData<PIN>,
}

/// Trait for each of the LE timer channels and their sets of 32 pins
pub trait LeTimerPin<const CN: u8> {
    /// Value to be written to LETIMERn_ROUTELOC0 register for the Pin implementing this trait
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
