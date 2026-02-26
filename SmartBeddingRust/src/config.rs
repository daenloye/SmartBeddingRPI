pub struct SystemConfig {
    pub debug_mode: bool,

    pub storage_enabled: bool,
    pub storage_path: &'static str,

    pub acceleration_period_ms: u64,
    pub acceleration_trigger_ms: u64,

    pub scan_delay_ms: u64,
    pub pressure_trigger_ms: u64,
    pub pressure_threshold: u16,
    pub pressure_matrix_visualization:bool,

    pub environment_period_ms: u64,
    pub environment_trigger_ms: u64,
    
}

pub const CONFIG: SystemConfig = SystemConfig {
    debug_mode: true,

    storage_enabled: true,
    storage_path: "./data_storage",

    acceleration_period_ms: 25,
    acceleration_trigger_ms: 50,  

    scan_delay_ms: 25,
    pressure_trigger_ms: 1000,
    pressure_threshold: 100,
    pressure_matrix_visualization: false,

    environment_period_ms: 10000,
    environment_trigger_ms: 20000,

};