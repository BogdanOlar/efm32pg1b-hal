//! Build with `cargo build --example spi_lcd --features="defmt"`
//!            `cargo build --example spi_lcd --features="defmt" --release`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

use embedded_graphics::{
    geometry::Point,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyle},
};
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt::assert_eq;
use defmt_rtt as _;

use ls013b7dh03::{prelude::*, WIDTH};

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();
    let clocks = p
        .cmu
        .split()
        .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div4);
    let gpio = p.gpio.split();

    // Let this App take control of display (this is a `UG154: EFM32 Pearl Gecko Starter Kit` paticularity)
    let _ = gpio.pd15.into_output().with_push_pull().build().set_high();

    let mut spi = p.usart1.into_spi_bus(
        gpio.pc8.into_output().with_push_pull().build(),
        gpio.pc6.into_output().with_push_pull().build(),
        gpio.pc7.into_input().with_filter().build(),
        SPIMODE,
    );
    let spi_br = spi.set_baudrate(1.MHz(), &clocks);
    // assert_eq!(spi_br.unwrap(), 1055555.Hz::<1, 1>());

    let cs = gpio.pd14.into_output().with_push_pull().build();
    let mut led0 = gpio.pf4.into_output().with_push_pull().build();
    let disp_com = gpio.pd13.into_output().with_push_pull().build();

    let mut buffer = [0u8; BUF_SIZE];
    let mut disp = Ls013b7dh03::new(spi, cs, led0, &mut buffer);

    let (tim0ch0, tim0ch1, _tim0ch2, _tim0ch3) =
        p.timer0.into_timer(TimerDivider::Div1024).into_channels();

    let mut com_inv = tim0ch1.into_pwm(disp_com);
    let ret_pwm = com_inv.set_duty_cycle(10);

    let mut delay_frames = tim0ch0.into_delay(&clocks);

    let mut counter = 0;
    const COM_INV_DELAY_MS: u32 = 16;
    const DRAW_DELAY_LOOP_COUNT_MAX: u32 = 1000 / COM_INV_DELAY_MS;

    enum DrawCmd {
        Full,
        ClearFull,
        Mod,
        ClearMod,
        WorstMod,
        ClearWorstMod,
        Idle,
    }

    let mut dm = DrawCmd::Full;

    loop {
        delay_frames.delay_ms(COM_INV_DELAY_MS);
        // blocking delay of 16ms
        // com_inv_delay.delay_ms(COM_INV_DELAY_MS);
        // disp.enable();
        // com_inv_delay.delay_us(2);
        // disp.disable();

        // Update display once in a while
        if counter < DRAW_DELAY_LOOP_COUNT_MAX {
            counter += 1;
        } else {
            counter = 0;

            match dm {
                DrawCmd::Full => {
                    for y in 0..HEIGHT as u8 {
                        for x in 0..WIDTH as u8 {
                            let write_ret = disp.write(x, y, true);
                            assert!(write_ret.is_ok());
                        }
                    }

                    // Update the display
                    disp.flush();
                    dm = DrawCmd::ClearFull;
                }
                DrawCmd::ClearFull => {
                    for y in 0..HEIGHT as u8 {
                        for x in 0..WIDTH as u8 {
                            let write_ret = disp.write(x, y, false);

                            assert!(write_ret.is_ok());
                        }
                    }

                    // Update the display
                    disp.flush();
                    dm = DrawCmd::Mod;
                }
                DrawCmd::Mod => {
                    for y in 0..HEIGHT as u8 {
                        for x in 0..WIDTH as u8 {
                            if y != 126 {
                                let write_ret = disp.write(x, y, true);
                                assert!(write_ret.is_ok());
                            } else {
                                let write_ret = disp.write(x, y, false);
                                assert!(write_ret.is_ok());
                            }
                        }
                    }
                    // Update the display
                    disp.flush();
                    dm = DrawCmd::ClearMod;
                }
                DrawCmd::ClearMod => {
                    for y in 0..HEIGHT as u8 {
                        for x in 0..WIDTH as u8 {
                            let write_ret = disp.write(x, y, false);

                            assert!(write_ret.is_ok());
                        }
                    }

                    disp.flush();
                    dm = DrawCmd::WorstMod;
                }
                DrawCmd::WorstMod => {
                    for y in 0..HEIGHT as u8 {
                        for x in 0..WIDTH as u8 {
                            if (y % 2) == 0 {
                                let write_ret = disp.write(x, y, true);
                                assert!(write_ret.is_ok());
                            } else {
                                let write_ret = disp.write(x, y, false);
                                assert!(write_ret.is_ok());
                            }
                        }
                    }
                    // Update the display
                    disp.flush();
                    dm = DrawCmd::ClearWorstMod;
                }
                DrawCmd::ClearWorstMod => {
                    for y in 0..HEIGHT as u8 {
                        for x in 0..WIDTH as u8 {
                            let write_ret = disp.write(x, y, false);

                            assert!(write_ret.is_ok());
                        }
                    }

                    disp.flush();
                    dm = DrawCmd::Idle;
                }
                DrawCmd::Idle => {
                    dm = DrawCmd::Full;
                }
            }
        }
    }
}
