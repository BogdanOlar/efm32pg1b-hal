use crate::gpio::{Output, Pin};
use cortex_m::asm::nop;
use efm32pg1b_pac::{
    cmu::{hfclksel::HF, hfclkstatus::SELECTED},
    Cmu,
};
use fugit::HertzU32;

/// Default HF RCO frequency at POR
const DEFAULT_HF_RCO_FREQUENCY: HertzU32 = HertzU32::MHz(19);

/// Default AUX HF RCO frequency at POR
const DEFAULT_AUX_HF_RCO_FREQUENCY: HertzU32 = HertzU32::MHz(19);

/// Default LF RCO frequency at POR
const DEFAULT_LF_RCO_FREQUENCY: HertzU32 = HertzU32::kHz(32);

/// Extension trait to split the CMU peripheral into clocks
pub trait CmuExt {
    /// The parts to split the CMU into
    type Parts;

    /// TODO:
    fn split(self) -> Self::Parts;
}

impl CmuExt for Cmu {
    type Parts = Clocks;

    fn split(self) -> Self::Parts {
        Clocks::calculate_hf_clocks(DEFAULT_HF_RCO_FREQUENCY)
    }
}

/// TODO:
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

impl Clocks {
    pub fn with_hf_clk(self, clk_src: HfClockSource, prescaler: u8) -> Self {
        let cmu = unsafe { Cmu::steal() };

        // Save the previous HF Clock source
        // [PANIC]: the reset value of the `SELECTED` field is `0x01`, so the field value cannot evaluate to something
        //          other than the enum
        let prev_hf_clk = cmu.hfclkstatus().read().selected().variant().unwrap();

        let hf_src_clk_freq = match clk_src {
            HfClockSource::HfXO(freq) => {
                // Enable HF XO
                cmu.oscencmd().write(|w| w.hfxoen().set_bit());

                // wait for HF XO clock to be stable
                while cmu.status().read().hfxordy().bit_is_clear() {
                    nop();
                }

                // select to HF XO
                cmu.hfclksel().write(|w| w.hf().variant(HF::Hfxo));

                freq
            }
            HfClockSource::HfRco => {
                // Enable HF RCO
                cmu.oscencmd().write(|w| w.hfrcoen().set_bit());

                // wait for HF RCO clock to be stable
                while cmu.status().read().hfrcordy().bit_is_clear() {
                    nop();
                }

                // select to HF RCO
                cmu.hfclksel().write(|w| w.hf().variant(HF::Hfrco));

                DEFAULT_HF_RCO_FREQUENCY
            }
            HfClockSource::LfXO(freq) => {
                // Enable LF XO
                cmu.oscencmd().write(|w| w.lfxoen().set_bit());

                // wait for LF XO clock to be stable
                while cmu.status().read().lfxordy().bit_is_clear() {
                    nop();
                }

                // select to LF XO
                cmu.hfclksel().write(|w| w.hf().variant(HF::Lfxo));

                freq
            }
            HfClockSource::LfRco => {
                // Enable LF RCO
                cmu.oscencmd().write(|w| w.lfrcoen().set_bit());

                // wait for LF RCO clock to be stable
                while cmu.status().read().lfrcordy().bit_is_clear() {
                    nop();
                }

                // select to LF RCO
                cmu.hfclksel().write(|w| w.hf().variant(HF::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
        };

        // The new HF Clock source
        // [PANIC]: the reset value of the `SELECTED` field is `0x01`, so the field value cannot evaluate to something
        //          other than the enum
        let cur_hf_clk = cmu.hfclkstatus().read().selected().variant().unwrap();

        // Disable the previously enabled HF Source Clk, if not the same as the currently enabled
        if prev_hf_clk != cur_hf_clk {
            match prev_hf_clk {
                SELECTED::Hfrco => cmu.oscencmd().write(|w| w.hfrcodis().set_bit()),
                SELECTED::Hfxo => cmu.oscencmd().write(|w| w.hfxodis().set_bit()),

                // FIXME: handle this contraint when implementing EMU
                // See 10.5.14 CMU_OSCENCMD - Oscillator Enable/Disable Command Register
                // WARNING: Do not disable the LFRCO if this oscillator is selected as the source for HFCLK.
                //          When waking up from EM4 make sure EM4UNLATCH in EMU_CMD is set for this to take effect
                SELECTED::Lfrco => cmu.oscencmd().write(|w| w.lfrcodis().set_bit()),

                // FIXME: handle this contraint when implementing EMU
                // See 10.5.14 CMU_OSCENCMD - Oscillator Enable/Disable Command Register
                // WARNING: Do not disable the LFXO if this oscillator is selected as the source for HFCLK.
                //          When waking up from EM4 make sure EM4UNLATCH in EMU_CMD is set for this to take effect
                SELECTED::Lfxo => cmu.oscencmd().write(|w| w.lfxodis().set_bit()),
            }
        }

        // Only 5 bits for prescaler
        assert!(prescaler <= 0b11111u8);

        // set prescaler
        cmu.hfpresc()
            .write(|w| unsafe { w.presc().bits(prescaler) });

        Self::calculate_hf_clocks(hf_src_clk_freq)
    }

    pub fn with_dbg_clk(self, clk_src: DbgClockSource) -> Self {
        let cmu = unsafe { Cmu::steal() };

        let dbg_clk_freq = match clk_src {
            DbgClockSource::AuxHfRco => {
                // check if Aux High Frequency RCO is enabled
                if cmu.status().read().auxhfrcoens().bit_is_clear() {
                    // Enable HF RCO
                    cmu.oscencmd().write(|w| w.auxhfrcoen().set_bit());
                }

                // wait for AUX HF RCO clock to be stable
                while cmu.status().read().auxhfrcordy().bit_is_clear() {
                    nop();
                }

                // select to LF RCO
                cmu.dbgclksel().write(|w| w.dbg().auxhfrco());

                DEFAULT_AUX_HF_RCO_FREQUENCY
            }
            DbgClockSource::HfClk => {
                // select to HF Clock as the Debug Clock
                cmu.dbgclksel().write(|w| w.dbg().hfclk());

                // the HF Bus Clock is the only one derived from HF Clock which dos not have a prescaler
                self.hf_bus_clk
            }
        };

        Self::calculate_hf_clocks(dbg_clk_freq)
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

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DbgClockSource {
    /// High Frequency Rco ()
    AuxHfRco,
    /// High Frequency Clock (i.e. the prescaled High Frequency Source Clock)
    HfClk,
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
