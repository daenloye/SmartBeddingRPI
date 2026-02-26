use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::Serialize;
use crate::pressure::{COL_SIZE, ROW_SIZE};
use crate::config::CONFIG; // <--- Importamos la configuración centralizada

#[derive(Serialize, Clone, Default)]
pub struct AudioMetrics {
    pub db_avg: f32,
    pub db_max: f32,
    pub db_min: f32,
    pub zcr: f32,
    pub crest_factor: f32,
    pub silence_percent: f32,
}

#[derive(Serialize, Clone)]
pub struct PressureSample {
    pub timestamp: String,
    pub measure: Arc<[[u16; COL_SIZE]; ROW_SIZE]>,
}

#[derive(Serialize, Clone)]
pub struct AccelSample {
    pub timestamp: String,
    pub measure: [f32; 6],
}

#[derive(Serialize, Clone)]
pub struct EnvironmentSample {
    pub timestamp: String,
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Serialize, Clone, Default)]
pub struct DataRaw {
    pub pressure: Vec<PressureSample>,
    pub acceleration: Vec<AccelSample>,
    pub environment: Vec<EnvironmentSample>,
}

#[derive(Serialize, Clone, Default)]
pub struct Measures {
    pub audio: Option<AudioMetrics>,
}

#[derive(Serialize)]
pub struct SessionSchema {
    pub initTimestamp: String,
    pub finishTimestamp: String,
    pub dataRaw: DataRaw,
    pub measures: Measures,
}

pub struct Storage;

impl Storage {
    pub fn init_path() -> PathBuf {
        // 1. Usamos la ruta definida en config.rs
        let base_path = Path::new(CONFIG.storage_path);
        
        // 2. Verificamos si el almacenamiento está habilitado en la config
        if !CONFIG.storage_enabled {
            println!("[STORAGE] ¡Alerta! El almacenamiento está deshabilitado en la configuración.");
        }

        if !base_path.exists() {
            fs::create_dir_all(base_path).expect("No se pudo crear el directorio base de storage");
        }

        let mut max_idx = 0;
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(s) = name.strip_prefix("register_") {
                        if let Ok(n) = s.parse::<u32>() { 
                            if n > max_idx { max_idx = n; } 
                        }
                    }
                }
            }
        }

        let new_path = base_path.join(format!("register_{}", max_idx + 1));
        
        // Creamos la subcarpeta para esta sesión específica
        fs::create_dir_all(&new_path).expect("Error creando la subcarpeta de registro");
        
        if CONFIG.debug_mode {
            println!("[STORAGE] Nueva sesión iniciada en: {}", new_path.display());
        }

        new_path
    }
}