//! Build with `cargo build --example spi --features="defmt"`

#![no_main]
#![no_std]

use cortex_m::asm::nop;
use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();

    let p = pac::Peripherals::take().unwrap();

    let cmu = unsafe { efm32pg1b_hal::pac::Cmu::steal() };
    let selected_hf_clk = cmu.hfclkstatus().read().selected().variant();
    defmt::println!("{}", selected_hf_clk);

    // Safety startup delay, in case the clock test goes wrong
    defmt::println!("Safe start");
    for _ in 0..1_000_000 {
        nop();
    }
    defmt::println!("Safe end");

    let clocks = p
        .cmu
        .split()
        .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div10)
        .with_dbg_clk(DbgClockSource::HfClk);

    // FIXME: Core clocks >= 25MHz require flash waitstates of at least `WS1` or `WS1SCBTP` set to `MSC_READCTRL.MODE`
    // let clocks = p
    //     .cmu
    //     .split()
    //     .with_hf_clk(HfClockSource::HfXO(40.MHz()), 10)
    //     .with_dbg_clk(DbgClockSource::HfClk);

    // FIXME: the RTT (defmt) can't be used when setting this source clock. Maybe AUX HFRCO has something to do with it?
    // let clocks = p
    //     .cmu
    //     .split()
    //     .with_hf_clk(HfClockSource::LfRco, 0)
    //     .with_dbg_clk(DbgClockSource::HfClk);

    // FIXME: the RTT (defmt) can't be used when setting this source clock. Maybe AUX HFRCO has something to do with it?
    // let clocks = p
    //     .cmu
    //     .split()
    //     .with_hf_clk(HfClockSource::LfXO(32_768.Hz()), 0)
    //     .with_dbg_clk(DbgClockSource::HfClk);

    defmt::println!("Clocks: {}", clocks);
    let selected_hf_clk = cmu.hfclkstatus().read().selected().variant();
    defmt::println!("{}", selected_hf_clk);

    loop {}
}
