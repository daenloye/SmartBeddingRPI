pub struct SystemConfig {
    pub debug_mode: bool,
    pub storage_enabled: bool,
    pub storage_path: &'static str,

    // Audio
    pub audio_sample_rate: u32,
    pub audio_channels: u16,
    pub audio_block_duration_s: u64,
    pub audio_silence_threshold: f32,
    // Sensores
    pub acceleration_period_ms: u64,
    pub acceleration_trigger_ms: u64,
    pub scan_delay_ms: u64,
    pub pressure_trigger_ms: u64,
    pub pressure_threshold: u16,
    pub pressure_matrix_visualization: bool,
    pub environment_period_ms: u64,
    pub environment_trigger_ms: u64,

    //Api
    pub api_token: &'static str, // Token estático para autenticación
}

pub const CONFIG: SystemConfig = SystemConfig {
    debug_mode: true,
    storage_enabled: true,
    storage_path: "/home/gibic/PruebaEnC/SmartBeddingRust/data_storage",

    audio_sample_rate: 44100,
    audio_channels: 2,
    audio_block_duration_s: 60,
    audio_silence_threshold: 0.01,

    acceleration_period_ms: 25,
    acceleration_trigger_ms: 50,  

    scan_delay_ms: 25,
    pressure_trigger_ms: 1000,
    pressure_threshold: 100,
    pressure_matrix_visualization: false,

    environment_period_ms: 10000,
    environment_trigger_ms: 20000,

    api_token: "1RlpMh35mILv48o0ElcywcpxX72bTjk9qiFqLwMYK33W4VSRQkm0IvsgolsS5Q9ETAb56uE3ZIG2UrR5S8lz4Ou6p90Sx9rA78WfrYe7t2C1QzfNHi1BCMmElTw4AmHXxiBAVHgIMEWuCupozWprh9KhY9GWOhwe65NhMoehgq5PB51m9SpwMSEfOLX7BiTsCw7NaMvY",
};