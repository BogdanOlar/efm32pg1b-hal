//! Build with `cargo build --example gpio`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use panic_halt as _;

#[entry]
fn main() -> ! {
    const PF4PIN: u16 = 4;
    const PF5PIN: u8 = 5;
    const PF4MODE_SHIFT: u8 = 16;
    const PF5MODE_SHIFT: u8 = 20;
    const PFXMODE_PUSH_PULL: u32 = 4;
    const PFCMODE_MASK: u32 = 0b1111;
    const COUNT_RESET_VAL: u32 = (u16::MAX as u32) * 2;
    let mut count = COUNT_RESET_VAL;

    let mut is_high = false;
    let cp = cortex_m::Peripherals::take().unwrap();
    let p = efm32pg1b_pac::Peripherals::take().unwrap();

    // Enable GPIO
    p.CMU.hfbusclken0.write(|w| w.gpio().set_bit());

    // Set pins direction
    p.GPIO.pf_model.write(|w| {
        // WARNING: using w multiple times will reset all its bits
        w
            // Enable PF4 as pull-up output
            .mode4()
            .variant(efm32pg1b_pac::gpio::pf_model::MODE4_A::PUSHPULL)
            // Enable PF5 as pull-up output
            .mode5()
            .variant(efm32pg1b_pac::gpio::pf_model::MODE5_A::PUSHPULL)
    });

    loop {
        if count == 0 {
            is_high = !is_high;

            // Blink LED on PF4
            match is_high {
                true => p
                    .GPIO
                    .pf_dout
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << PF4PIN)) }),
                false => p
                    .GPIO
                    .pf_dout
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << PF4PIN)) }),
            }

            count = COUNT_RESET_VAL;
        }
        count -= 1;
    }
}
