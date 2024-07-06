//! Build with `cargo build --example spi --features="defmt"`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use efm32pg1b_hal::prelude::*;

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
use defmt::{assert_eq, println};
use defmt_rtt as _;

use ls013b7dh03::{Ls013b7dh03, HEIGHT, SPIMODE};

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();

    let p = pac::Peripherals::take().unwrap();

    let clocks = p.cmu.split();

    let gpio = p.gpio.split();

    let tx = gpio.pc6.into_output().with_push_pull().build();
    let rx = gpio.pc7.into_input().with_filter().build();
    let clk = gpio.pc8.into_output().with_push_pull().build();

    let mut board_disp_enable = gpio.pd15.into_output().with_push_pull().build();

    // Let this App take control of display
    let _ = board_disp_enable.set_high();

    // let mut btn0 = gpio.pf6.into_input().build();

    let mut spi = p.usart1.into_spi_bus(clk, tx, rx, SPIMODE);
    let spi_br = spi.set_baudrate(1.MHz(), &clocks);
    // assert_eq!(spi_br.unwrap(), 1055555.Hz::<1, 1>());
    let cs = gpio.pd14.into_output().with_push_pull().build();
    let disp_com = gpio.pd13.into_output().with_push_pull().build();

    let mut buffer: [u8; 2304] = [0; 2304];
    let mut disp = Ls013b7dh03::new(spi, cs, disp_com, &mut buffer);

    let mut poor_mans_timer: u32 = 0;
    let mut tgl = true;
    let mut counter = 0;
    let mut ypos = 0;

    // FIXME: this whole thing only works in Debug builds :D
    loop {
        if poor_mans_timer >= (136_666 / 19) {
            poor_mans_timer = 0;

            tgl = !tgl;

            match tgl {
                true => disp.enable(),
                false => disp.disable(),
            }

            counter += 1;

            if counter >= 10 {
                counter = 0;

                for x in 10..100 {
                    disp.write(x, ypos, false);
                }
                ypos += 1;
                for x in 10..100 {
                    disp.write(x, ypos, true);
                }

                ypos = ypos % HEIGHT as u8;

                disp.flush();
            }
        } else {
            poor_mans_timer += 1;
        }
    }
}
