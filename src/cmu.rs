//! Clock Management Unit
//!

use crate::gpio::{pin::mode::OutputMode, pin::Pin};
use cortex_m::asm::nop;
use efm32pg1b_pac::{
    cmu::vals::{Dbg, Hf, Hfclklepresc, HfprescPresc, Lfa, Lfb, Lfe, Selected},
    cryotimer::vals::Oscsel,
    wdog::vals::Clksel,
    CMU, CRYOTIMER, WDOG,
};

/// Default HF RCO frequency at Reset, in Hz
const DEFAULT_HF_RCO_FREQUENCY: u32 = 19_000_000;

/// Default AUX HF RCO frequency at Reset, in Hz
const DEFAULT_AUX_HF_RCO_FREQUENCY: u32 = 19_000_000;

/// Default LF RCO frequency at Reset, in Hz
const DEFAULT_LF_RCO_FREQUENCY: u32 = 32_768;

/// Default Ultra LF RCO frequency at Reset, in Hz
const DEFAULT_ULF_RCO_FREQUENCY: u32 = 1;

/// Extension trait to split the CMU peripheral into clocks
pub trait CmuExt {
    /// The parts to split the CMU into
    type Parts;

    /// TODO:
    fn split(self) -> Self::Parts;
}

impl CmuExt for efm32pg1b_pac::cmu::Cmu {
    type Parts = Clocks;

    fn split(self) -> Self::Parts {
        Clocks::calculate_hf_clocks(DEFAULT_HF_RCO_FREQUENCY)
    }
}

/// TODO:
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Clocks {
    /// High Frequency Peripheral Clock, in Hz
    hf_per_clk: u32,

    /// High Frequency Core Clock, in Hz
    hf_core_clk: u32,

    /// High Frequency Export Clock, in Hz
    hf_exp_clk: u32,

    /// High Frequency  Bus Clock, in Hz
    hf_bus_clk: u32,

    /// Low Frequency A Clock, in Hz
    lfa_clk: Option<u32>,

    /// Low Frequency B Clock, in Hz
    lfb_clk: Option<u32>,

    /// Low Frequency E Clock, in Hz
    lfe_clk: Option<u32>,

    /// Watch Dog Clock, in Hz
    wdog_clk: Option<u32>,

    /// Cryo Timer Clock, in Hz
    cryo_clk: Option<u32>,
}

impl Clocks {
    /// High Frequency Peripheral Clock
    pub fn hf_per_clk(&self) -> u32 {
        self.hf_per_clk
    }

    /// High Frequency Core Clock
    pub fn hf_core_clk(&self) -> u32 {
        self.hf_core_clk
    }

    /// High Frequency Export Clock
    pub fn hf_exp_clk(&self) -> u32 {
        self.hf_exp_clk
    }

    /// High Frequency  Bus Clock
    pub fn hf_bus_clk(&self) -> u32 {
        self.hf_bus_clk
    }

    /// Low Frequency A Clock
    pub fn lfa_clk(&self) -> Option<u32> {
        self.lfa_clk
    }

    /// Low Frequency B Clock
    pub fn lfb_clk(&self) -> Option<u32> {
        self.lfb_clk
    }

    /// Low Frequency E Clock
    pub fn lfe_clk(&self) -> Option<u32> {
        self.lfe_clk
    }

    /// Watch Dog Clock
    pub fn wdog_clk(&self) -> Option<u32> {
        self.wdog_clk
    }

    /// Cryo Timer Clock
    pub fn cryo_clk(&self) -> Option<u32> {
        self.cryo_clk
    }

    /// TODO:
    pub fn with_hf_clk(self, clk_src: HfClockSource, prescaler: HfClockPrescaler) -> Self {
        let prev_hf_clk = CMU.hfclkstatus().read().selected();

        let hf_src_clk_freq = match clk_src {
            HfClockSource::HfXO(freq) => {
                // Enable HF XO
                CMU.oscencmd().write(|w| w.set_hfxoen(true));

                // wait for HF XO clock to be stable
                while CMU.status().read().hfxordy() == false {
                    nop();
                }

                // select to HF XO
                CMU.hfclksel().write(|w| w.set_hf(Hf::Hfxo));

                freq
            }
            HfClockSource::HfRco => {
                // Enable HF RCO
                CMU.oscencmd().write(|w| w.set_hfrcoen(true));

                // wait for HF RCO clock to be stable
                while CMU.status().read().hfrcordy() == false {
                    nop();
                }

                // select to HF RCO
                CMU.hfclksel().write(|w| w.set_hf(Hf::Hfrco));

                DEFAULT_HF_RCO_FREQUENCY
            }
            HfClockSource::LfXO(freq) => {
                // Enable LF XO
                CMU.oscencmd().write(|w| w.set_lfxoen(true));

                // wait for LF XO clock to be stable
                while CMU.status().read().lfxordy() == false {
                    nop();
                }

                // select to LF XO
                CMU.hfclksel().write(|w| w.set_hf(Hf::Lfxo));

                freq
            }
            HfClockSource::LfRco => {
                // Enable LF RCO
                CMU.oscencmd().write(|w| w.set_lfrcoen(true));

                // wait for LF RCO clock to be stable
                while CMU.status().read().lfrcordy() == false {
                    nop();
                }

                // select to LF RCO
                CMU.hfclksel().write(|w| w.set_hf(Hf::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
        };

        // The new HF Clock source
        // [PANIC]: the reset value of the `SELECTED` field is `0x01`, so the field value cannot evaluate to something
        //          other than the enum
        let cur_hf_clk = CMU.hfclkstatus().read().selected();

        // Disable the previously enabled HF Source Clk, if not the same as the currently enabled
        if prev_hf_clk != cur_hf_clk {
            match prev_hf_clk {
                Selected::Hfrco => CMU.oscencmd().write(|w| w.set_hfrcodis(true)),
                Selected::Hfxo => CMU.oscencmd().write(|w| w.set_hfxodis(true)),

                // FIXME: handle this contraint when implementing EMU
                // See 10.5.14 CMU_OSCENCMD - Oscillator Enable/Disable Command Register
                // WARNING: Do not disable the LFRCO if this oscillator is selected as the source for HFCLK.
                //          When waking up from EM4 make sure EM4UNLATCH in EMU_CMD is set for this to take effect
                Selected::Lfrco => CMU.oscencmd().write(|w| w.set_lfrcodis(true)),

                // FIXME: handle this contraint when implementing EMU
                // See 10.5.14 CMU_OSCENCMD - Oscillator Enable/Disable Command Register
                // WARNING: Do not disable the LFXO if this oscillator is selected as the source for HFCLK.
                //          When waking up from EM4 make sure EM4UNLATCH in EMU_CMD is set for this to take effect
                Selected::Lfxo => CMU.oscencmd().write(|w| w.set_lfxodis(true)),
                _ => {}
            };
        }

        // set prescaler
        CMU.hfpresc()
            .write(|w| w.set_presc(HfprescPresc::from_bits(prescaler as u8)));

        Self::calculate_hf_clocks(hf_src_clk_freq)
    }

    /// TODO:
    pub fn with_dbg_clk(self, clk_src: DbgClockSource) -> Self {
        
        let dbg_clk_freq = match clk_src {
            DbgClockSource::AuxHfRco => {
                // check if Aux High Frequency RCO is enabled
                if CMU.status().read().auxhfrcoens() == false {
                    // Enable HF RCO
                    CMU.oscencmd().write(|w| w.set_auxhfrcoen(true));
                }

                // wait for AUX HF RCO clock to be stable
                while CMU.status().read().auxhfrcordy() == false {
                    nop();
                }

                // select to LF RCO
                CMU.dbgclksel().write(|w| w.set_dbg(Dbg::Auxhfrco));

                DEFAULT_AUX_HF_RCO_FREQUENCY
            }
            DbgClockSource::HfClk => {
                // select to HF Clock as the Debug Clock
                CMU.dbgclksel().write(|w| w.set_dbg(Dbg::Hfclk));

                // the HF Bus Clock is the only one derived from HF Clock which dos not have a prescaler
                self.hf_bus_clk
            }
        };

        Self::calculate_hf_clocks(dbg_clk_freq)
    }

    /// TODO:
    pub fn with_lfa_clk(self, clk_src: LfClockSource) -> Self {
        
        // The bus interface to the Low Energy A Peripherals is clocked by HFBUSCLKLE and this clock therefore needs to
        // be enabled when programming a Low Energy (LE) peripheral.
        self.enable_hf_bus_clk_le();

        let lfa_clk_freq = match clk_src {
            LfClockSource::LfXO(freq) => {
                // Ensure Low Frequency XO is enabled
                self.enable_lfxo_clock();

                // select LF XO
                CMU.lfaclksel().write(|w| w.set_lfa(Lfa::Lfxo));

                freq
            }
            LfClockSource::LfRco => {
                // Ensure Low Frequency RCO is enabled
                self.enable_lfrco_clock();

                // select LF RCO
                CMU.lfaclksel().write(|w| w.set_lfa(Lfa::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
            LfClockSource::UlfRco => {
                // select ULF RCO
                CMU.lfaclksel().write(|w| w.set_lfa(Lfa::Ulfrco));

                DEFAULT_ULF_RCO_FREQUENCY
            }
        };

        Self {
            lfa_clk: Some(lfa_clk_freq),
            ..self
        }
    }

    /// TODO:
    pub fn with_lfb_clk(self, clk_src: LfBClockSource) -> Self {
        
        let lfb_clk_freq = match clk_src {
            LfBClockSource::LfXO(freq) => {
                // Ensure Low Frequency XO is enabled
                self.enable_lfxo_clock();

                // select LF XO
                CMU.lfbclksel().write(|w| w.set_lfb(Lfb::Lfxo));

                freq
            }
            LfBClockSource::LfRco => {
                // Ensure Low Frequency RCO is enabled
                self.enable_lfrco_clock();

                // Select LF RCO
                CMU.lfbclksel().write(|w| w.set_lfb(Lfb::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
            LfBClockSource::UlfRco => {
                // Select ULF RCO
                CMU.lfbclksel().write(|w| w.set_lfb(Lfb::Ulfrco));

                DEFAULT_ULF_RCO_FREQUENCY
            }
            LfBClockSource::HfClkLe(is_div_4) => {
                // Set High Frequency Clock LE prescaler
                let freq = match is_div_4 {
                    true => {
                        CMU.hfpresc().modify(|w| w.set_hfclklepresc(Hfclklepresc::Div4));
                        self.hf_bus_clk / 4
                    }
                    false => {
                        CMU.hfpresc().modify(|w| w.set_hfclklepresc(Hfclklepresc::Div2));
                        self.hf_bus_clk / 2
                    }
                };

                // Select High Frequency Clock LE
                CMU.lfbclksel().write(|w| w.set_lfb(Lfb::Hfclkle));

                freq
            }
        };

        // The bus interface to the Low Energy A Peripherals is clocked by HFBUSCLKLE and this clock therefore needs to
        // be enabled when programming a Low Energy (LE) peripheral.
        self.enable_hf_bus_clk_le();

        Self {
            lfb_clk: Some(lfb_clk_freq),
            ..self
        }
    }

    /// TODO:
    pub fn with_lfe_clk(self, clk_src: LfClockSource) -> Self {
        
        let lfe_clk_freq = match clk_src {
            LfClockSource::LfXO(freq) => {
                // Ensure Low Frequency XO is enabled
                self.enable_lfxo_clock();

                // select LF XO
                CMU.lfeclksel().write(|w| w.set_lfe(Lfe::Lfxo));

                freq
            }
            LfClockSource::LfRco => {
                // Ensure Low Frequency RCO is enabled
                self.enable_lfrco_clock();

                // select LF RCO
                CMU.lfeclksel().write(|w| w.set_lfe(Lfe::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
            LfClockSource::UlfRco => {
                // select ULF RCO
                CMU.lfeclksel().write(|w| w.set_lfe(Lfe::Ulfrco));

                DEFAULT_ULF_RCO_FREQUENCY
            }
        };

        // The bus interface to the Low Energy A Peripherals is clocked by HFBUSCLKLE and this clock therefore needs to
        // be enabled when programming a Low Energy (LE) peripheral.
        self.enable_hf_bus_clk_le();

        Self {
            lfe_clk: Some(lfe_clk_freq),
            ..self
        }
    }

    /// TODO:
    pub fn with_wdog_clk(self, clk_src: LfClockSource) -> Self {
        
        let wdog_clk_freq = match clk_src {
            LfClockSource::LfXO(freq) => {
                // Ensure Low Frequency XO is enabled
                self.enable_lfxo_clock();

                // select LF XO
                WDOG.ctrl().modify(|w| w.set_clksel(Clksel::Lfxo));

                freq
            }
            LfClockSource::LfRco => {
                // Ensure Low Frequency RCO is enabled
                self.enable_lfrco_clock();

                // select LF RCO
                WDOG.ctrl().modify(|w| w.set_clksel(Clksel::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
            LfClockSource::UlfRco => {
                // select ULF RCO
                WDOG.ctrl()
                    .modify(|w| w.set_clksel(Clksel::Ulfrco));

                DEFAULT_ULF_RCO_FREQUENCY
            }
        };

        Self {
            wdog_clk: Some(wdog_clk_freq),
            ..self
        }
    }

    /// TODO:
    pub fn with_cryo_clk(self, clk_src: LfClockSource) -> Self {
        
        let cryo_clk_freq = match clk_src {
            LfClockSource::LfXO(freq) => {
                // Ensure Low Frequency XO is enabled
                self.enable_lfxo_clock();

                // select LF XO
                CRYOTIMER.ctrl().modify(|w| w.set_oscsel(Oscsel::Lfxo));

                freq
            }
            LfClockSource::LfRco => {
                // Ensure Low Frequency RCO is enabled
                self.enable_lfrco_clock();

                // select LF RCO
                CRYOTIMER.ctrl().modify(|w| w.set_oscsel(Oscsel::Lfrco));

                DEFAULT_LF_RCO_FREQUENCY
            }
            LfClockSource::UlfRco => {
                // select ULF RCO
                CRYOTIMER.ctrl().modify(|w| w.set_oscsel(Oscsel::Ulfrco));

                DEFAULT_ULF_RCO_FREQUENCY
            }
        };

        Self {
            cryo_clk: Some(cryo_clk_freq),
            ..self
        }
    }

    fn calculate_hf_clocks(hf_src_clk: u32) -> Self {
        
        //  clock divider for the HFPERCLK (relative to HFCLK).
        let hf_clk_prescaler: u32 = CMU.hfpresc().read().presc().to_bits() as u32;
        let hf_clk_prescaler = hf_clk_prescaler + 1;
        let hf_clk = hf_src_clk / hf_clk_prescaler;

        let hf_per_clk_prescaler: u32 = CMU.hfperpresc().read().presc().to_bits() as u32;
        let hf_per_clk_prescaler = hf_per_clk_prescaler + 1;
        let hf_per_clk = hf_clk / hf_per_clk_prescaler;

        let hf_core_clk_prescaler: u32 = CMU.hfcorepresc().read().presc().to_bits() as u32;
        let hf_core_clk_prescaler = hf_core_clk_prescaler + 1;
        let hf_core_clk = hf_clk / hf_core_clk_prescaler;

        let hf_exp_clk_prescaler: u32 = CMU.hfexppresc().read().presc().to_bits() as u32;
        let hf_exp_clk_prescaler = hf_exp_clk_prescaler + 1;
        let hf_exp_clk = hf_clk / hf_exp_clk_prescaler;

        let hf_bus_clk = hf_clk;

        Clocks {
            hf_per_clk,
            hf_core_clk,
            hf_exp_clk,
            hf_bus_clk,
            lfa_clk: None,
            lfb_clk: None,
            lfe_clk: None,
            wdog_clk: None,
            cryo_clk: None,
        }
    }

    /// Set to enable the clock for LE. Interface used for bus access to Low Energy peripherals.
    fn enable_hf_bus_clk_le(&self) {
        
        // Enable High Frequency Clock LE
        CMU.hfbusclken0().modify(|w| w.set_le(true));
    }

    /// Enable Low Frequency XO
    fn enable_lfxo_clock(&self) {
                // Ensure Low Frequency XO is enabled
        if CMU.status().read().lfxoens() == false {
            CMU.oscencmd().write(|w| w.set_lfxoen(true));
        }

        // wait for LF XO clock to be stable
        while CMU.status().read().lfxordy() == false {
            nop();
        }
    }

    /// Enable Low Frequency RCO
    fn enable_lfrco_clock(&self) {
                // Ensure Low Frequency RCO is enabled
        if CMU.status().read().lfrcoens() == false {
            CMU.oscencmd().write(|w| w.set_lfrcoen(true));
        }

        // wait for LF RCO clock to be stable
        while CMU.status().read().lfrcordy() == false {
            nop();
        }
    }
}

/// TODO:
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HfClockSource {
    /// High Frequency external oscillator, outputting the given declared frequency in Hz
    HfXO(u32),
    /// High Frequency Rco
    HfRco,
    /// Low Frequency external oscillator, outputting the given declared frequency in Hz
    LfXO(u32),
    /// Low Frequency Rco
    LfRco,
}

/// High Frequency Clock divider values
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HfClockPrescaler {
    /// Divider by 1
    Div1,
    /// Divider by 2
    Div2,
    /// Divider by 3
    Div3,
    /// Divider by 4
    Div4,
    /// Divider by 5
    Div5,
    /// Divider by 6
    Div6,
    /// Divider by 7
    Div7,
    /// Divider by 8
    Div8,
    /// Divider by 9
    Div9,
    /// Divider by 10
    Div10,
    /// Divider by 11
    Div11,
    /// Divider by 12
    Div12,
    /// Divider by 13
    Div13,
    /// Divider by 14
    Div14,
    /// Divider by 15
    Div15,
    /// Divider by 16
    Div16,
    /// Divider by 17
    Div17,
    /// Divider by 18
    Div18,
    /// Divider by 19
    Div19,
    /// Divider by 20
    Div20,
    /// Divider by 21
    Div21,
    /// Divider by 22
    Div22,
    /// Divider by 23
    Div23,
    /// Divider by 24
    Div24,
    /// Divider by 25
    Div25,
    /// Divider by 26
    Div26,
    /// Divider by 27
    Div27,
    /// Divider by 28
    Div28,
    /// Divider by 29
    Div29,
    /// Divider by 30
    Div30,
    /// Divider by 31
    Div31,
    /// Divider by 32
    Div32,
}

/// TODO:
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DbgClockSource {
    /// High Frequency Rco ()
    AuxHfRco,
    /// High Frequency Clock (i.e. the prescaled High Frequency Source Clock)
    HfClk,
}

/// Low Frequency clocks sources (used for LFACLK, LFECLK, WDOGCLK, CRYOCLK)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LfClockSource {
    /// Low Frequency External Oscillator
    LfXO(u32),

    /// Low Frequency Rco
    LfRco,

    /// Ultra Low Frequency Rco
    UlfRco,
}

/// High and Low Frequency clock sources for LFBCLK only
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LfBClockSource {
    /// High Frequency Clock Low Energy (this is a prescaled HFCLK: if the bool is `false` then
    /// the divider is `2`, otherwise `4`)
    HfClkLe(bool),

    /// Low Frequency External Oscillator
    LfXO(u32),

    /// Low Frequency Rco
    LfRco,

    /// Ultra Low Frequency Rco
    UlfRco,
}

/// TODO:
pub trait CmuPin0 {
    /// TODO:
    fn loc(&self) -> u8;
}

macro_rules! impl_clock_0_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<MODE> CmuPin0 for Pin<$port, $pin, MODE>
        where
            MODE: OutputMode,
        {
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

/// TODO:
pub trait CmuPin1 {
    /// TODO:
    fn loc(&self) -> u8;
}

macro_rules! impl_clock_1_loc {
    ($loc:literal, $port:literal, $pin:literal) => {
        impl<MODE> CmuPin1 for Pin<$port, $pin, MODE>
        where
            MODE: OutputMode,
        {
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
