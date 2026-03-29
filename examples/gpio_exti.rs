//! Build with `cargo build --example gpio_exti --features="defmt"`

#![no_main]
#![no_std]

use core::cell::RefCell;

use cortex_m_rt::entry;
use critical_section::Mutex;
use defmt::info;
use defmt_rtt as _;
use efm32pg1b_hal::gpio::dynamic::DynamicPin;
use efm32pg1b_hal::gpio::exti::{ExtiBind, ExtiEdge};
use efm32pg1b_hal::pac::{interrupt, Interrupt, NVIC};
use efm32pg1b_hal::{
    gpio::{
        exti::{self, ExtiId},
        pin::PinInfo,
    },
    prelude::*,
};
use panic_probe as _;

static PIN_DBG: Mutex<RefCell<Option<DynamicPin>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();

    let mut gpio = Gpio::new(p.gpio);

    gpio.port_f.set_drive_strength(DriveStrength::Strong);
    gpio.port_f.set_drive_strength_alt(DriveStrength::Strong);

    // Calling this is fine since the debug pins use the `Primary` not the `Alternate` port `F` ctrl configs
    gpio.port_f.set_din_dis_alt(DataInCtrl::Disabled);

    let mut pin_dbg = gpio.pa0.into_mode::<OutPp>().into_dynamic_pin();
    let _ = pin_dbg.set_low();
    critical_section::with(|cs| {
        PIN_DBG.borrow_ref_mut(cs).replace(pin_dbg);
    });

    // LED 0, BTN 0
    let mut led0 = gpio.pf4.into_mode::<OutPp>();
    let mut btn0 = gpio.pf6.into_mode::<InFloat>();

    // LED 1, BTN 1
    let mut led1 = gpio.pf5.into_mode::<OutPpAlt>();
    let mut btn1 = gpio.pf7.into_mode::<InFilt>();

    // ---- Btn 0 ----
    let mut btn0_exti = gpio.exti6ctrl.bind(&btn0);
    info!("{} bound to {}", btn0_exti.id(), btn0);
    for exti in ExtiId::Exti0 as u8..=ExtiId::Exti15 as u8 {
        info!(
            "\tExti{} bound to {}",
            exti,
            exti::mmio::exti_bind_get(exti.try_into().unwrap())
        );
    }
    btn0_exti.edge_select(ExtiEdge::Falling);
    btn0_exti.enable();

    // ---- Btn 1 ----

    let exti = ExtiId::Exti4;
    let port = btn1.port();
    let pin = btn1.pin();

    let bind_ret = exti::mmio::exti_bind(exti, port, pin);
    info!("binding {} to {} {}: {}", exti, port, pin, bind_ret);

    for exti in ExtiId::Exti0 as u8..=ExtiId::Exti15 as u8 {
        info!(
            "\tExti{} bound to {}",
            exti,
            exti::mmio::exti_bind_get(exti.try_into().unwrap())
        );
    }
    exti::mmio::exti_edge_select(exti, ExtiEdge::Falling);
    exti::mmio::exti_enable(exti);

    // ---- NVIC ----
    unsafe {
        NVIC::unmask(Interrupt::GPIO_EVEN);
        NVIC::unmask(Interrupt::GPIO_ODD);
    }

    // button states
    let mut btn0_prev = true;
    let mut btn1_prev = true;

    loop {
        // Button 0 and LED 0
        if let Ok(btn0_cur) = btn0.is_high() {
            if btn0_cur != btn0_prev {
                defmt::info!("btn0 {}: {}", &btn0, !btn0_cur);
                led0.toggle().unwrap();
                let ledstate = led0.is_set_high().unwrap();
                defmt::info!("led0 {}: {}", &led0, ledstate);
                btn0_prev = btn0_cur;
            }
        }

        // Button 1 and LED 1
        if let Ok(btn1_cur) = btn1.is_high() {
            if btn1_cur != btn1_prev {
                defmt::info!("btn1 {}: {}", &btn1, !btn1_cur);
                led1.toggle().unwrap();
                let ledstate = led1.is_set_high().unwrap();
                defmt::info!("led1 {}: {}", &led1, ledstate);
                btn1_prev = btn1_cur;
            }
        }
    }
}

#[interrupt]
fn GPIO_EVEN() {
    // Set a debug pin to high in order to see how much time this interrupt takes
    critical_section::with(|cs| {
        let mut binding = PIN_DBG.borrow(cs).borrow_mut();
        let b = binding.as_mut();
        if let Some(p) = b {
            let _ = p.set_high();
        }
    });

    for exti in exti::mmio::exti_flags_even() {
        if let Some(edge) = exti::mmio::exti_edge_get(exti) {
            match edge {
                ExtiEdge::Rising => {
                    info!("GPIO_EVEN: {} rising", exti);
                    exti::mmio::exti_edge_select(exti, ExtiEdge::Falling);
                }
                ExtiEdge::Falling => {
                    info!("GPIO_EVEN: {} falling", exti);
                    exti::mmio::exti_edge_select(exti, ExtiEdge::Rising);
                }
                ExtiEdge::Both => {
                    info!("GPIO_EVEN: {} both", exti);
                }
            }
        }

        exti::mmio::exti_clear(exti);
    }

    // Clear the debug pin
    critical_section::with(|cs| {
        let mut binding = PIN_DBG.borrow(cs).borrow_mut();
        let b = binding.as_mut();
        if let Some(p) = b {
            let _ = p.set_low();
        }
    });
}

#[interrupt]
fn GPIO_ODD() {
    for exti in exti::mmio::exti_flags_odd() {
        if let Some(edge) = exti::mmio::exti_edge_get(exti) {
            match edge {
                ExtiEdge::Rising => {
                    exti::mmio::exti_edge_select(exti, ExtiEdge::Falling);
                    info!("GPIO_ODD: {} rising", exti);
                }
                ExtiEdge::Falling => {
                    exti::mmio::exti_edge_select(exti, ExtiEdge::Rising);
                    info!("GPIO_ODD: {} falling", exti);
                }
                ExtiEdge::Both => {
                    info!("GPIO_ODD: {} both", exti);
                }
            }
        }

        exti::mmio::exti_clear(exti);
    }
}
