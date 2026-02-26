pub struct SystemConfig {
    pub debug_mode: bool,
    pub storage_enabled: bool,
    pub scan_delay_ms: u64,
    pub main_trigger_ms: u64,     // <--- Solo añadimos esto
    pub pressure_threshold: u16,
    pub storage_path: &'static str,
}

pub const CONFIG: SystemConfig = SystemConfig {
    debug_mode: false,            // Lo pongo en false para que no te sature el "Ocupado"
    storage_enabled: true,
    scan_delay_ms: 50,            
    main_trigger_ms: 1000,        // <--- Aquí configuras cada cuánto quieres ver la matriz (ej. 1000ms)
    pressure_threshold: 100,
    storage_path: "./data_logs",
};