use efm32pg1b_pac::Cmu;
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
        let cmu = unsafe { Cmu::steal() };
        let hf_src_clk = HertzU32::MHz(19);

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

impl Clocks {}
