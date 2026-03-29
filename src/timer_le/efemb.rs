//! Embassy time driver implementation for LeTimer
//!

#[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
use crate::gpio::{
    erased::ErasedPin,
    pin::mode::{Out, PushPull},
    OutPp,
};
use crate::{
    pac::{interrupt, Cmu, Interrupt, NVIC},
    timer_le::mmio::{self, Command, InterruptFlag},
};
use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use critical_section::{CriticalSection, Mutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
#[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
use embedded_hal::digital::OutputPin;

/// Embassy time driver using LeTimer0 peripheral
pub struct Ticker {
    /// Number of times the timer counter overflowed
    /// LeTimer is actually count-down, but we're calling this `overflow` just to avoid confusion
    ovf_count: AtomicU32,
    /// Initialization flag. If the ticker is not initialized, then [`Ticker::now()`] will just return `0`
    is_init: AtomicBool,
    /// Waker queue
    queue: Mutex<RefCell<Queue>>,
    /// Debug GPIO pins
    #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
    dbg_pins: Mutex<RefCell<Option<DbgPins>>>,
}

embassy_time_driver::time_driver_impl!(static EFEMB_TIME_DRIVER: Ticker = Ticker{
    ovf_count:AtomicU32::new(0),
    is_init:AtomicBool::new(false),
    queue: Mutex::new(RefCell::new(Queue::new())),
    #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
    dbg_pins: Mutex::new(RefCell::new(None))
});

impl Ticker {
    /// Initialize the embassy time driver.
    ///
    /// WARNING: LfAClk needs to be enabled, and the HfClk source clock needs to be one of the HF sources (i.e. not LF),
    ///          otherwise you'll get a bus fault.
    ///
    ///For example, if you want the timer driver to run at 32.768 kHz, make sure to enable the
    /// `efemb-timdrv-letim0-hz-32_768` crate feature, and initialize the clock sources
    ///
    /// ```rust,norun
    /// let _clocks = p
    ///     .cmu
    ///     .split()
    ///     .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div1)
    ///     .with_lfa_clk(LfClockSource::LfRco);
    /// ```
    /// For a 1 kKz timer, enable the `efemb-timdrv-letim0-hz-1_000` feature, and configure the LfAClk
    /// ```rust,norun
    /// let _clocks = p
    ///     // ...
    ///     .with_lfa_clk(LfClockSource::UlfRco);
    /// ```
    pub fn init() {
        // Stop the timer if it's running
        if mmio::running() {
            mmio::cmd(Command::Stop);
        }

        // Enable LE Timer
        let cmu = unsafe { Cmu::steal() };
        cmu.lfaclken0().modify(|_, w| w.letimer0().set_bit());

        mmio::reset();
        mmio::comp1_set(0);
        mmio::ienable(InterruptFlag::Underflow);
        mmio::ienable(InterruptFlag::Comp0);

        #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
        {
            use crate::gpio::Gpio;

            let p = unsafe { crate::pac::Peripherals::steal() };
            let gpio = Gpio::new(p.gpio);

            let pins = DbgPins {
                sched: gpio.pa0.into_mode::<OutPp>().into_erased_pin(),
                isr: gpio.pa1.into_mode::<OutPp>().into_erased_pin(),
                isr_ovf: gpio.pa2.into_mode::<OutPp>().into_erased_pin(),
                isr_comp: gpio.pa3.into_mode::<OutPp>().into_erased_pin(),
            };
            critical_section::with(|cs| {
                EFEMB_TIME_DRIVER.dbg_pins.replace(cs, Some(pins));
            })
        }

        // Enable the timer interrupt
        unsafe {
            NVIC::unmask(Interrupt::LETIMER0);
        }

        // start the timer
        mmio::cmds(&[Command::Clear, Command::Start]);

        // Mark the timer as initialized
        EFEMB_TIME_DRIVER.is_init.store(true, Ordering::Relaxed);
    }

    /// Handle LeTimer0 interrupt
    pub fn on_interrupt(&self) {
        critical_section::with(|cs| {
            #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
            let mut pins_bound = self.dbg_pins.borrow(cs).borrow_mut();
            #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
            {
                if let Some(pins) = pins_bound.as_mut() {
                    let _ = pins.isr.set_high();
                }
            }

            if mmio::if_get(InterruptFlag::Underflow) {
                self.ovf_count.fetch_add(1, Ordering::Relaxed);
                mmio::if_clear(InterruptFlag::Underflow);
                #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
                {
                    if let Some(pins) = pins_bound.as_mut() {
                        let _ = pins.isr_ovf.set_high();
                    }
                }
            }

            if mmio::if_get(InterruptFlag::Comp0) {
                mmio::if_clear(InterruptFlag::Comp0);
                #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
                {
                    if let Some(pins) = pins_bound.as_mut() {
                        let _ = pins.isr_comp.set_high();
                    }
                }
            }

            if mmio::if_get(InterruptFlag::Comp1) {
                mmio::if_clear(InterruptFlag::Comp1);
                mmio::idisable(InterruptFlag::Comp1);
            }

            let mut queue = self.queue.borrow(cs).borrow_mut();
            loop {
                let next = queue.next_expiration(self.checked_now(&cs));

                if self.set_alarm(&cs, self.checked_now(&cs), next) {
                    break;
                }
            }

            #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
            {
                if let Some(pins) = pins_bound.as_mut() {
                    let _ = pins.isr.set_low();
                    let _ = pins.isr_ovf.set_low();
                    let _ = pins.isr_comp.set_low();
                }
            }
        });
    }

    /// Set an interrupt to trigger at the given `next` logical time, relative to the current `now` logical time
    fn set_alarm(&self, _cs: &CriticalSection, now: u64, next: u64) -> bool {
        if now >= next {
            // alarm has already expired
            false
        } else {
            let now_count = (now & u16::MAX as u64) as u16;
            let rem_of_count = u16::MAX - now_count;
            let dif = next - now;

            if dif < rem_of_count as u64 {
                // alarm will occur before next timer overflow
                let cnt = next % u16::MAX as u64;

                mmio::comp0_set(cnt as u16);
                mmio::ienable(InterruptFlag::Comp0);
            } else {
                // we'll set the alarm on one of the next timer overflows
                mmio::idisable(InterruptFlag::Comp0);
            }

            true
        }
    }

    /// Get the current logical time while in a critical section.
    ///
    /// Useful because while in a critical section [`Ticker::ovf_count`] will not be incremented, so calling
    /// [`Driver::now()`] would return an incorect (smaller) time, whic breaks the 'monotonic' timer spec.
    ///
    /// This will increment the overflow counter is the underflow flag is set.
    fn checked_now(&self, _cs: &CriticalSection) -> u64 {
        let ovf = if mmio::if_get(InterruptFlag::Underflow) {
            mmio::if_clear(InterruptFlag::Underflow);
            self.ovf_count.fetch_add(1, Ordering::Relaxed) + 1
        } else {
            self.ovf_count.load(Ordering::Relaxed)
        };

        let counter = mmio::counter_get();
        (ovf as u64) << u16::BITS | counter as u64
    }
}

#[interrupt]
fn LETIMER0() {
    EFEMB_TIME_DRIVER.on_interrupt();
}

impl Driver for Ticker {
    fn now(&self) -> u64 {
        if self.is_init.load(Ordering::Relaxed) {
            loop {
                let ovf_before = self.ovf_count.load(Ordering::SeqCst);
                let counter = mmio::counter_get();
                let ovf = self.ovf_count.load(Ordering::SeqCst);
                if ovf_before == ovf {
                    break (ovf as u64) << u16::BITS | counter as u64;
                }
            }
        } else {
            // Ticker not initialized
            0
        }
    }

    fn schedule_wake(&self, at: u64, waker: &core::task::Waker) {
        critical_section::with(|cs| {
            #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
            let mut pins_bound = self.dbg_pins.borrow(cs).borrow_mut();
            #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
            {
                if let Some(pins) = pins_bound.as_mut() {
                    let _ = pins.sched.set_high();
                }
            }

            let mut queue = self.queue.borrow(cs).borrow_mut();
            if queue.schedule_wake(at, waker) {
                // Use the Comp1 interrupt flag to trigger the [`Ticker::on_interrupt()`] execution, which will handle
                // setting the timer alarm.
                mmio::ienable(InterruptFlag::Comp1);
                mmio::if_set(InterruptFlag::Comp1);
            }

            #[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
            {
                if let Some(pins) = pins_bound.as_mut() {
                    let _ = pins.sched.set_low();
                }
            }
        });
    }
}

/// Debug pins used for debugging LeTimer0 when used as the embassy time [`Driver`]
#[cfg(feature = "efemb-timdrv-letim0-dbg-pins")]
pub struct DbgPins {
    /// PA0 held high while [`Driver.schedule_wake()`] is being executed
    sched: ErasedPin<Out<PushPull>>,
    /// PA1 held high while [`Ticker.on_interrupt()`] is being executed
    isr: ErasedPin<Out<PushPull>>,
    /// PA2 held high while [`Ticker.on_interrupt()`] is being executed, and the timer overflow interrupt flag has been set
    isr_ovf: ErasedPin<Out<PushPull>>,
    /// PA3 held high while [`Ticker.on_interrupt()`] is being executed, and the timer comp0 interrupt flag has been set
    isr_comp: ErasedPin<Out<PushPull>>,
}
