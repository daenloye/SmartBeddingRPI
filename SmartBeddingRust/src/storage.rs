use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::Serialize;
use crate::pressure::{COL_SIZE, ROW_SIZE};

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

#[derive(Serialize)]
pub struct SessionSchema {
    pub initTimestamp: String,
    pub finishTimestamp: String,
    pub dataRaw: DataRaw,
    pub audioMetrics: Option<AudioMetrics>,
}

pub struct Storage;

impl Storage {
    pub fn init_path() -> PathBuf {
        let base_path = Path::new("/home/gibic/PruebaEnC/SmartBeddingRust/data_storage");
        if !base_path.exists() {
            fs::create_dir_all(base_path).ok();
        }

        let mut max_idx = 0;
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(s) = name.strip_prefix("register_") {
                        if let Ok(n) = s.parse::<u32>() { if n > max_idx { max_idx = n; } }
                    }
                }
            }
        }

        let new_path = base_path.join(format!("register_{}", max_idx + 1));
        fs::create_dir_all(&new_path).expect("Error creando carpeta");
        new_path
    }
}