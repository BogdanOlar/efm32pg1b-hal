//! Build with `cargo build --example gpio`

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use panic_halt as _;

#[entry]
fn main() -> ! {
    let mut count = 0;

    loop {
        count += 1;

        if count % 10 == 0 {
            count += 2;
        }
    }
}
