use crate::gpio::{Output, Pin};
use efm32pg1b_pac::{
    cmu::{hfclksel::HF, hfpresc::PRESC},
    Cmu,
};
use fugit::HertzU32;

/// Extension trait to split the CMU peripheral into clocks
pub trait CmuExt {
    /// The parts to split the CMU into
    type Parts;

    /// TODO:
    fn split(self) -> Self::Parts;
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Clocks {
    /// High Frequency Peripheral Clock
    pub hf_per_clk: HertzU32,

    /// High Frequency Core Clock
    pub hf_core_clk: HertzU32,

    /// High Frequency Export Clock
    pub hf_exp_clk: HertzU32,

    /// High Frequency  Bus Clock
    pub hf_bus_clk: HertzU32,
}

impl CmuExt for Cmu {
    type Parts = Clocks;

    fn split(self) -> Self::Parts {
        // FIXME: this assumes HFRCO clock source @ 19 MHz
        let hf_src_clk = HertzU32::MHz(19);

        Clocks::calculate_hf_clocks(hf_src_clk)
    }
}

pub enum HfClockSource {
    /// High Frequency external oscillator, outputting the given declared frequency
    HfXO(HertzU32),
    /// High Frequency Rco
    HfRco,
    /// Low Frequency external oscillator, outputting the given declared frequency
    LfXO(HertzU32),
    /// Low Frequency Rco
    LfRco,
}

impl Clocks {
    pub fn with_hf_clk(self, clk_src: HfClockSource, prescaler: u8) -> Self {
        let cmu = unsafe { Cmu::steal() };
        let hf_src_clk;

        match clk_src {
            HfClockSource::HfXO(freq) => {
                hf_src_clk = freq;
                todo!();
            }
            HfClockSource::HfRco => {
                // Enable HF RCO
                cmu.oscencmd().write(|w| w.hfrcoen().set_bit());

                // wait for HF RCO clock to be stable
                while cmu.status().read().hfrcordy().bit_is_clear() {}

                // select to HF RCO
                cmu.hfclksel().write(|w| w.hf().variant(HF::Hfrco));

                hf_src_clk = HertzU32::MHz(19);
            }
            HfClockSource::LfXO(freq) => {
                hf_src_clk = freq;
                todo!();
            }
            HfClockSource::LfRco => todo!(),
        }

        // Only 5 bits for prescaler
        assert!(prescaler <= 0b11111u8);

        // set prescaler
        cmu.hfpresc()
            .write(|w| unsafe { w.presc().bits(prescaler) });

        Self::calculate_hf_clocks(hf_src_clk)
    }

    fn calculate_hf_clocks(hf_src_clk: HertzU32) -> Self {
        let cmu = unsafe { Cmu::steal() };

        //  clock divider for the HFPERCLK (relative to HFCLK).
        let hf_clk_prescaler: u32 = cmu.hfpresc().read().presc().bits().into();
        let hf_clk_prescaler = hf_clk_prescaler + 1;
        let hf_clk = hf_src_clk / hf_clk_prescaler;

        let hf_per_clk_prescaler: u32 = cmu.hfperpresc().read().presc().bits().into();
        let hf_per_clk_prescaler = hf_per_clk_prescaler + 1;
        let hf_per_clk = hf_clk / hf_per_clk_prescaler;

        let hf_core_clk_prescaler: u32 = cmu.hfcorepresc().read().presc().bits().into();
        let hf_core_clk_prescaler = hf_core_clk_prescaler + 1;
        let hf_core_clk = hf_clk / hf_core_clk_prescaler;

        let hf_exp_clk_prescaler: u32 = cmu.hfexppresc().read().presc().bits().into();
        let hf_exp_clk_prescaler = hf_exp_clk_prescaler + 1;
        let hf_exp_clk = hf_clk / hf_exp_clk_prescaler;

        let hf_bus_clk = hf_clk;

        Clocks {
            hf_per_clk,
            hf_core_clk,
            hf_exp_clk,
            hf_bus_clk,
        }
    }
}

pub trait CmuPin0 {
    fn loc(&self) -> u8;
}

macro_rules! impl_clock_0_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<ANY> CmuPin0 for Pin<$port, $pin, Output<ANY>> {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_clock_0_loc!(0, 'A', 1);
impl_clock_0_loc!(1, 'B', 15);
impl_clock_0_loc!(2, 'C', 6);
impl_clock_0_loc!(3, 'C', 11);
impl_clock_0_loc!(4, 'D', 9);
impl_clock_0_loc!(5, 'D', 14);
impl_clock_0_loc!(6, 'F', 2);
impl_clock_0_loc!(7, 'F', 7);

pub trait CmuPin1 {
    fn loc(&self) -> u8;
}

macro_rules! impl_clock_1_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<ANY> CmuPin1 for Pin<$port, $pin, Output<ANY>> {
            fn loc(&self) -> u8 {
                $loc
            }
        }
    };
}

impl_clock_1_loc!(0, 'A', 0);
impl_clock_1_loc!(1, 'B', 14);
impl_clock_1_loc!(2, 'C', 7);
impl_clock_1_loc!(3, 'C', 10);
impl_clock_1_loc!(4, 'D', 10);
impl_clock_1_loc!(5, 'D', 15);
impl_clock_1_loc!(6, 'F', 3);
impl_clock_1_loc!(7, 'F', 6);
