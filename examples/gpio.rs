//! Build with `cargo build --example gpio`

#![no_main]
#![no_std]

use core::convert::TryInto;

use cortex_m_rt::entry;
use efm32pg1b_pac as pac;
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    const PF4PIN: u8 = 4;
    const PF5PIN: u8 = 5;
    const PF6PIN: u8 = 6;
    const PF7PIN: u8 = 7;

    const PF4_PIN_MASK: u32 = 1 << PF4PIN;
    const PF5_PIN_MASK: u32 = 1 << PF5PIN;
    const PF6_PIN_MASK: u32 = 1 << PF6PIN;
    const PF7_PIN_MASK: u32 = 1 << PF7PIN;

    let core_p = cortex_m::Peripherals::take().unwrap();
    let mut p = pac::Peripherals::take().unwrap();

    defmt::println!("Hello, world!");

    // Enable GPIO
    p.CMU.hfbusclken0().write(|w| w.gpio().set_bit());

    //
    // Led0 -> PF4
    //

    // Set pins direction
    p.GPIO.pf_model().modify(|_, w| {
        w.mode4().variant(pac::gpio::pf_model::MODE4::Pushpull);
        w
    });

    //
    // Led1 -> PF5
    //

    // Set pins direction
    p.GPIO.pf_model().modify(|_, w| {
        w.mode5().variant(pac::gpio::pf_model::MODE5::Pushpull);
        w
    });

    //
    // Button0 -> PF6
    //

    // Set as input with pull resistor
    p.GPIO.pf_model().modify(|_, w| {
        w.mode6().variant(pac::gpio::pf_model::MODE6::Inputpull);
        w
    });
    // set direction of pull to Up
    p.GPIO.pf_dout().modify(|r, w| unsafe {
        let x = r.bits() | PF6_PIN_MASK;
        w.dout().bits(x.try_into().unwrap())
    });

    //
    // Button1 -> PF7
    //
    // Set as input. TODO: make this with pullup as well?
    p.GPIO.pf_model().modify(|_, w| {
        w.mode7().variant(pac::gpio::pf_model::MODE7::Input);
        w
    });

    loop {
        let din_reg_value = p.GPIO.pf_din().read().bits();

        if (din_reg_value & PF6_PIN_MASK) == 0 {
            p.GPIO
                .pf_dout()
                .modify(|r, w| unsafe { w.bits(r.bits() | PF4_PIN_MASK) });
        } else {
            p.GPIO
                .pf_dout()
                .modify(|r, w| unsafe { w.bits(r.bits() & !PF4_PIN_MASK) });
        }

        if (din_reg_value & PF7_PIN_MASK) == 0 {
            p.GPIO
                .pf_dout()
                .modify(|r, w| unsafe { w.bits(r.bits() | PF5_PIN_MASK) });
        } else {
            p.GPIO
                .pf_dout()
                .modify(|r, w| unsafe { w.bits(r.bits() & !PF5_PIN_MASK) });
        }
    }
}
