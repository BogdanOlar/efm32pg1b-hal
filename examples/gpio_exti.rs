#![no_main]
#![no_std]

use core::cell::RefCell;
use cortex_m::asm::nop;
use cortex_m_rt::entry;
use critical_section::Mutex;
use defmt::info;
use defmt_rtt as _;
use efm32pg1b_hal::{
    gpio::{
        dynamic::DynamicPin,
        exti::{self, ExtiEdge, ExtiId},
    },
    pac::{Interrupt, NVIC},
    prelude::*,
    timer_le::efemb::Ticker,
};
// @note: `use embassy_time` is required in some form in order for defmt timestamps provided by `embassy-time` to work
use embassy_time::Timer as _;
use panic_probe as _;

static LED0: Mutex<RefCell<Option<DynamicPin>>> = Mutex::new(RefCell::new(None));
static LED1: Mutex<RefCell<Option<DynamicPin>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();

    // Initialize the embassy time driver (for defmt timestamps)
    let _clocks = p.cmu.split().with_lfa_clk(LfClockSource::LfRco);
    Ticker::init();

    // ---- NVIC ----
    unsafe {
        NVIC::unmask(Interrupt::GPIO_EVEN);
        NVIC::unmask(Interrupt::GPIO_ODD);
    }

    let mut gpio = Gpio::new(p.gpio);

    gpio.port_f.set_drive_strength(DriveStrength::Strong);
    gpio.port_f.set_drive_strength_alt(DriveStrength::Strong);
    gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled);

    // ---- Led 0 ----
    critical_section::with(|cs| {
        LED0.borrow(cs)
            .borrow_mut()
            .replace(gpio.pf4.into_mode::<OutPp>().into_dynamic_pin());
    });

    // ---- Btn 0 ----
    let mut btn0 = gpio
        .pf6
        .into_mode::<InFloat>()
        .into_exti_bound_pin(gpio.exti4ctrl, |exti| {
            critical_section::with(|cs| {
                let mut led = LED0.borrow(cs).borrow_mut();
                if let Some(led) = led.as_mut() {
                    exti_toggle(exti, led);
                }
            });
        });
    btn0.exti_ctrl_ref_mut().edge_select(ExtiEdge::Falling);
    btn0.exti_ctrl_ref_mut().enable();

    // ---- Led 1 ----
    critical_section::with(|cs| {
        LED1.borrow(cs)
            .borrow_mut()
            .replace(gpio.pf5.into_mode::<OutPpAlt>().into_dynamic_pin());
    });

    // ---- Btn 1 ----
    // we'll make btn1 pin a dynamic pin and try to bind it to ExtiCtrl<5>, which is a valid binding, so we can unwrap()
    let mut btn1 = gpio
        .pf7
        .into_mode::<InFilt>()
        .into_dynamic_pin()
        .try_into_exti_bound_pin(gpio.exti5ctrl, |exti| {
            critical_section::with(|cs| {
                let mut led = LED1.borrow(cs).borrow_mut();
                if let Some(led) = led.as_mut() {
                    exti_toggle(exti, led);
                }
            });
        })
        .unwrap();
    btn1.exti_ctrl_ref_mut().edge_select(ExtiEdge::Falling);
    btn1.exti_ctrl_ref_mut().enable();

    // ---- Print EXTI ----
    info!("External interrupts:");
    for exti in ExtiId::Exti0 as u8..=ExtiId::Exti15 as u8 {
        let exti = exti.try_into().unwrap();
        let en = exti::mmio::exti_is_enabled(exti);
        info!(
            "\t {} {} bound to {}",
            if en { "enabled" } else { "-------" },
            exti,
            exti::mmio::exti_bind_get(exti)
        );
    }

    loop {
        nop();
    }
}

fn exti_toggle(exti: ExtiId, led: &mut DynamicPin) {
    if let Some(edge) = exti::mmio::exti_edge_get(exti) {
        let new_edge = match edge {
            ExtiEdge::Rising => {
                info!("{} Released", exti);
                let _ = led.set_low();
                ExtiEdge::Falling
            }
            ExtiEdge::Falling => {
                info!("{} Pressed", exti);
                let _ = led.set_high();
                ExtiEdge::Rising
            }
            ExtiEdge::Both => {
                info!("{} Both", exti);
                edge
            }
        };

        exti::mmio::exti_edge_select(exti, new_edge);
    }
}
