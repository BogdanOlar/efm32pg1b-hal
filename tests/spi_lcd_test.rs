#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests(setup=rtt_target::rtt_init_defmt!())]
mod tests {
    use efm32pg1b_hal::{
        cmu::{CmuExt, HfClockPrescaler, HfClockSource},
        pac::Peripherals,
        spi::{Spi, UsartSpiExt},
    };
    use embedded_hal::digital::OutputPin;
    use fugit::RateExtU32;
    use ls013b7dh03::{
        spi::{Ls013b7dh03, SPIMODE},
        BUF_SIZE,
    };

    // // An optional init function which is called before every test
    // // Asyncness is optional, so is the return value
    // #[init]
    // fn init() -> Peripherals {
    //     let p = Peripherals::take().unwrap();
    //     let clocks = p
    //         .cmu
    //         .split()
    //         .with_hf_clk(HfClockSource::HfRco, HfClockPrescaler::Div4);

    //     let gpio = p.gpio.split();

    //     // Let this App take control of display (this is a `UG154: EFM32 Pearl Gecko Starter Kit` paticularity)
    //     let _ = gpio.pd15.into_output().with_push_pull().build().set_high();

    //     let mut spi = p.usart1.into_spi_bus(
    //         gpio.pc8.into_output().with_push_pull().build(),
    //         gpio.pc6.into_output().with_push_pull().build(),
    //         gpio.pc7.into_input().with_filter().build(),
    //         SPIMODE,
    //     );
    //     let spi_br = spi.set_baudrate(1.MHz(), &clocks);
    //     assert_eq!(spi_br.unwrap(), 1055555.Hz::<1, 1>());

    //     let cs = gpio.pd14.into_output().with_push_pull().build();
    //     let disp_com = gpio.pd13.into_output().with_push_pull().build();

    //     let mut buffer = [0u8; BUF_SIZE];
    //     let mut disp = Ls013b7dh03::new(spi, cs, disp_com, &mut buffer);

    //     p
    // }

    // Tests can be async (needs feature `embassy`)
    // Tests can take the state returned by the init function (optional)
    #[test]
    fn takes_state() {
        assert!(true)
    }

    // // Tests can be conditionally enabled (with a cfg attribute)
    // #[test]
    // #[cfg(feature = "log")]
    // fn log() {
    //     rtt_target::rtt_init_log!();
    //     log::info!("Hello, log!");
    //     assert!(true)
    // }

    // Tests can be ignored with the #[ignore] attribute
    #[test]
    #[ignore]
    fn it_works_ignored() {
        assert!(false)
    }

    // Tests can fail with a custom error message by returning a Result
    #[test]
    fn it_fails_with_err() -> Result<(), &'static str> {
        Err("It failed because ...")
    }

    // Tests can be annotated with #[should_panic] if they are expected to panic
    #[test]
    #[should_panic]
    fn it_passes() {
        assert!(false)
    }

    // Tests can be annotated with #[timeout(<secs>)] to change the default timeout of 60s
    #[test]
    #[timeout(10)]
    fn it_timeouts() {
        let x = 1;
        if x == 2 {}
        loop {} // should run into the 10s timeout
    }
}
