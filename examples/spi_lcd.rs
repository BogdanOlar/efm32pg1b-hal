//! Build with `cargo build --example spi_lcd --features="defmt"`

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

use ls013b7dh03::prelude::*;

#[entry]
fn main() -> ! {
    let _core_p = cortex_m::Peripherals::take().unwrap();
    let p = pac::Peripherals::take().unwrap();
    let clocks = p.cmu.split();
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
    assert_eq!(spi_br.unwrap(), 1055555.Hz::<1, 1>());

    let cs = gpio.pd14.into_output().with_push_pull().build();
    let disp_com = gpio.pd13.into_output().with_push_pull().build();

    let mut buffer = [0u8; BUF_SIZE];
    let mut disp = Ls013b7dh03::new(spi, cs, disp_com, &mut buffer);

    let (_tim0ch0, tim0ch1, _tim0ch2, _tim0ch3) =
        p.timer0.into_timer(TimerDivider::Div2).into_channels();

    let mut com_inv_delay = tim0ch1.into_delay(&clocks);

    let mut tgl = true;
    let mut counter = 0;
    let mut ypos: i32 = 0;

    loop {
        // blocking delay of 16ms
        com_inv_delay.delay_ms(16);

        // Toggle Comm Inversion pin
        tgl = match tgl {
            true => {
                disp.enable();
                !tgl
            }
            false => {
                disp.disable();
                !tgl
            }
        };

        // Update display once in a while
        if counter < 1 {
            counter += 1;
        } else {
            counter = 0;

            // erase old circle
            let circle = Circle::new(Point::new(22, ypos as i32), ypos as u32 + 5)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 2));
            let _ = circle.draw(&mut disp);

            ypos += 2;
            ypos = ypos % HEIGHT as i32;

            // draw new circle
            let circle = Circle::new(Point::new(22, ypos as i32), ypos as u32 + 5)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2));
            let _ = circle.draw(&mut disp);

            // Update the display
            disp.flush();
        }
    }
}
