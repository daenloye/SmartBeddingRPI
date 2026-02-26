use std::fs;
use std::path::{Path, PathBuf};
use crate::config::CONFIG;
use crate::pressure::{COL_SIZE, ROW_SIZE};

// Estructura para la matriz de presión (1Hz)
pub struct PressureSample {
    pub timestamp: String,
    pub matrix: [[u16; COL_SIZE]; ROW_SIZE],
}

// Estructura para la aceleración (20Hz)
pub struct AccelSample {
    pub timestamp: String,
    pub data: [f32; 6], // [gx, gy, gz, ax, ay, az]
}

// NUEVO: Estructura para sensores ambientales (0.05Hz / Cada 20s)
pub struct EnvironmentSample {
    pub timestamp: String,
    pub temperature: f32,
    pub humidity: f32,
}

pub struct Storage {
    pub current_dir: PathBuf,
    pub pressure_buffer: Vec<PressureSample>,
    pub accel_buffer: Vec<AccelSample>,
    pub env_buffer: Vec<EnvironmentSample>, // Nuevo buffer
}

impl Storage {
    pub fn init() -> Self {
        let base_path = Path::new(CONFIG.storage_path);
        if !base_path.exists() {
            fs::create_dir_all(base_path).expect("[STORAGE] No se pudo crear la carpeta base");
        }

        let mut max_index: u32 = 0;
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(os_name) = path.file_name() {
                        if let Some(name_str) = os_name.to_str() {
                            if let Some(stripped) = name_str.strip_prefix("register_") {
                                if let Ok(n) = stripped.parse::<u32>() {
                                    if n > max_index { max_index = n; }
                                }
                            }
                        }
                    }
                }
            }
        }

        let new_folder_name = format!("register_{}", max_index + 1);
        let new_path = base_path.join(new_folder_name);
        fs::create_dir(&new_path).expect("[STORAGE] No se pudo crear la carpeta de registro");

        if CONFIG.debug_mode {
            println!("[STORAGE] Sesión iniciada y buffers listos en memoria.");
        }

        Self {
            current_dir: new_path,
            pressure_buffer: Vec::with_capacity(100), 
            accel_buffer: Vec::with_capacity(2000), 
            env_buffer: Vec::with_capacity(20), // Capacidad para ~7 minutos de ambiente
        }
    }

    pub fn add_pressure_sample(&mut self, timestamp: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
        let ts_clone = timestamp.clone();
        self.pressure_buffer.push(PressureSample { timestamp, matrix });

        if CONFIG.debug_mode {
            println!("[STORAGE-P] [{}] Buffer presión: {}", ts_clone, self.pressure_buffer.len());
        }
    }

    pub fn add_accel_sample(&mut self, timestamp: String, data: [f32; 6]) {
        let ts_clone = timestamp.clone();
        self.accel_buffer.push(AccelSample { timestamp, data });

        if CONFIG.debug_mode && self.accel_buffer.len() % 20 == 0 {
            println!("[STORAGE-A] [{}] Buffer accel: {}", ts_clone, self.accel_buffer.len());
        }
    }

    // NUEVO: Método para añadir datos ambientales
    pub fn add_env_sample(&mut self, timestamp: String, temperature: f32, humidity: f32) {
        let ts_clone = timestamp.clone();
        self.env_buffer.push(EnvironmentSample { 
            timestamp, 
            temperature, 
            humidity 
        });

        if CONFIG.debug_mode {
            println!("[STORAGE-E] [{}] Buffer ambiente: {} (T: {:.2}, H: {:.2})", 
                     ts_clone, self.env_buffer.len(), temperature, humidity);
        }
    }
}