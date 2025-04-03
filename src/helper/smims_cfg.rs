use tabled::Tabled;

use crate::vlfd::cfg::CfgInfo;

// A single row in a vertical table that maps a configuration field to its value.
#[derive(Tabled)]
pub struct CfgTable {
    /// The name of the configuration field.
    field: &'static str,
    /// The value of the configuration field.
    value: String,
}

impl CfgTable {
    /// Creates a vertical representation (as a vector of rows) of any configuration that implements `CfgInfo`.
    ///
    /// Extend this list with all fields you wish to display.
    pub fn from_cfg(cfg: &impl CfgInfo) -> Vec<Self> {
        vec![
            Self { field: "Clock High Delay", value: format!("{:#04x}", cfg.get_vericomm_clock_highdelay()) },
            Self { field: "Clock Low Delay", value: format!("{:#04x}", cfg.get_vericomm_clock_lowdelay()) },
            Self { field: "Vericomm ISV", value: format!("{:#04x}", cfg.get_vericomm_isv()) },
            Self { field: "Vericomm Clockcheck Enable", value: format!("{:#04x}", if cfg.get_vericomm_clockcheck_enable() { 1 } else { 0 }) },
            Self { field: "Veri SDK Channel Selector", value: format!("{:#04x}", cfg.get_veri_sdk_channel_selector()) },
            Self { field: "Mode Selector", value: format!("{:#04x}", cfg.get_mode_selector()) },
            Self { field: "Flash Begin Block Addr", value: format!("{:#04x}", cfg.get_flash_begin_block_addr()) },
            Self { field: "Flash Begin Cluster Addr", value: format!("{:#04x}", cfg.get_flash_begin_cluster_addr()) },
            Self { field: "Flash Read End Block Addr", value: format!("{:#04x}", cfg.get_flash_read_end_block_addr()) },
            Self { field: "Flash Read End Cluster Addr", value: format!("{:#04x}", cfg.get_flash_read_end_cluster_addr()) },
            Self { field: "Security Key", value: format!("{:#04x}", cfg.get_security_key()) },
            Self { field: "SMIMS Version", value: format!("{:#04x}", cfg.smims_version()) },
            Self { field: "SMIMS Major Version", value: format!("{:#04x}", cfg.smims_majorversion()) },
            Self { field: "SMIMS Sub Version", value: format!("{:#04x}", cfg.smims_subversion()) },
            Self { field: "SMIMS Subsub Version", value: format!("{:#04x}", cfg.smims_subsubversion()) },
            Self { field: "FIFO Size", value: format!("{:#04x}", cfg.fifo_size()) },
            Self { field: "Flash Total Block", value: format!("{:#04x}", cfg.flash_total_block()) },
            Self { field: "Flash Block Size", value: format!("{:#04x}", cfg.flash_block_size()) },
            Self { field: "Flash Cluster Size", value: format!("{:#04x}", cfg.flash_cluster_size()) },
            Self { field: "Vericomm Ability", value: format!("{:#04x}", if cfg.vericomm_ability() { 1 } else { 0 }) },
            Self { field: "Veri Instrument Ability", value: format!("{:#04x}", if cfg.veri_instrument_ability() { 1 } else { 0 }) },
            Self { field: "Veri Link Ability", value: format!("{:#04x}", if cfg.veri_link_ability() { 1 } else { 0 }) },
            Self { field: "Veri SOC Ability", value: format!("{:#04x}", if cfg.veri_soc_ability() { 1 } else { 0 }) },
            Self { field: "Vericomm Pro Ability", value: format!("{:#04x}", if cfg.vericomm_pro_ability() { 1 } else { 0 }) },
            Self { field: "Veri SDK Ability", value: format!("{:#04x}", if cfg.veri_sdk_ability() { 1 } else { 0 }) },
            Self { field: "Is Programmed", value: format!("{:#04x}", if cfg.is_programmed() { 1 } else { 0 }) },
            Self { field: "Is PCB Connect", value: format!("{:#04x}", if cfg.is_pcb_connect() { 1 } else { 0 }) },
            Self { field: "Is Vericomm Clockcontinue", value: format!("{:#04x}", if cfg.is_vericomm_clockcontinue() { 1 } else { 0 }) },
        ]
    }
}