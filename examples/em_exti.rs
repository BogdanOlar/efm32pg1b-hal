#![no_main]
#![no_std]

use core::cell::RefCell;
use cortex_m::asm;
use cortex_m_rt::entry;
use critical_section::Mutex;
use defmt::info;
use defmt_rtt as _;
use efm32pg1b_hal::{
    gpio::exti::{self, ExtiEdge, ExtiId},
    pac::{Interrupt, NVIC},
    prelude::*,
    timer_le::efemb::Ticker,
};
use panic_probe as _;
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;

static BTN0_CHANNEL: Mutex<RefCell<Option<bool>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();

    // ---- NVIC ----
    unsafe {
        NVIC::unmask(Interrupt::GPIO_EVEN);
        NVIC::unmask(Interrupt::GPIO_ODD);
    }

    // Initialize the embassy time driver (for defmt timestamps)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    let mut gpio = Gpio::new(p.gpio);

    gpio.port_f.set_drive_strength(DriveStrength::Strong);
    gpio.port_f.set_drive_strength_alt(DriveStrength::Strong);
    gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled);

    // ---- Btn 0 ----
    let mut btn0 = gpio
        .pf6
        .into_mode::<InFloat>()
        .into_exti_bound_pin(gpio.exti4ctrl, btn0_exti_handler);
    btn0.exti_ctrl_ref_mut().edge_select(ExtiEdge::Falling);
    btn0.exti_ctrl_ref_mut().enable();

    loop {
        info!("Thread WAKE");

        // ---- Btn 0 ----
        if let Some(btn_transitioned_pressed) =
            critical_section::with(|cs| BTN0_CHANNEL.borrow(cs).borrow_mut().take())
        {
            match btn_transitioned_pressed {
                true => {
                    // Button was pressed
                    core_p.SCB.set_sleepdeep();
                    core_p.SCB.set_sleeponexit();

                    info!("deepsleep ON, sleeponexit ON");
                }
                false => {
                    // Button was released
                    core_p.SCB.clear_sleepdeep();
                    core_p.SCB.clear_sleeponexit();

                    info!("deepsleep OFF, sleeponexit OFF");
                }
            }
        } else {
            info!("Thread no event");
        }

        info!("Thread SLEEP");
        asm::dsb();
        asm::wfe();
    }
}

fn btn0_exti_handler(exti: ExtiId) {
    let cur_edge = exti::mmio::exti_edge_get(exti).unwrap();
    let mut event = None;

    let new_edge = match cur_edge {
        ExtiEdge::Falling => {
            info!("{} Pressed", exti);
            event = Some(true);
            ExtiEdge::Rising
        }
        ExtiEdge::Rising => {
            info!("{} Released", exti);
            event = Some(false);
            ExtiEdge::Falling
        }
        ExtiEdge::Both => {
            info!("{} Both", exti);
            cur_edge
        }
    };

    critical_section::with(|cs| {
        // create an event for the button channel
        BTN0_CHANNEL.borrow(cs).replace(event);

        // make sure the main thread will run
        //
        // If the SLEEPONEXIT bit of the SCR is set to 1, when the processor completes the execution of all exception
        // handlers it returns to Thread mode and immediately enters sleep mode. Use this mechanism in applications that
        // only require the processor to run when an exception occurs.
        unsafe { cortex_m::Peripherals::steal().SCB.clear_sleeponexit() };
        info!("sleeponexit OFF");

        // set next edge
        exti::mmio::exti_edge_select(exti, new_edge);
    })
}
