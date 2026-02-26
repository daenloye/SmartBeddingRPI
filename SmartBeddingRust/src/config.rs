/// Configuración global del sistema SmartBedding
/// Este módulo centraliza el comportamiento de todos los hilos.

pub struct SystemConfig {
    /// Modo Debug: Si es true, imprime logs detallados en consola
    pub debug_mode: bool,
    
    /// Modo Almacenamiento: Si es true, habilita el guardado en disco/CSV
    pub storage_enabled: bool,
    
    /// Intervalo de escaneo del hardware (ms)
    pub scan_delay_ms: u64,
    
    /// Umbral de presión para considerar que hay "alguien" (puntos calientes)
    pub pressure_threshold: u16,
    
    /// Ruta de la carpeta de almacenamiento
    pub storage_path: &'static str,
}

/// Instancia única de configuración
pub const CONFIG: SystemConfig = SystemConfig {
    debug_mode: true,             // Cambiar a false en producción
    storage_enabled: true,        // Habilita el módulo storage.rs
    scan_delay_ms: 50,            // Frecuencia del hilo de hardware
    pressure_threshold: 100,      // Valor para el renderizado verde y detección
    storage_path: "./data_logs",
};