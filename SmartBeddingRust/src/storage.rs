use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::Serialize;
use crate::config::CONFIG;
use crate::pressure::{COL_SIZE, ROW_SIZE};
use chrono::Local;

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

#[derive(Serialize, Clone)]
pub struct DataRaw {
    pub pressure: Vec<PressureSample>,
    pub acceleration: Vec<AccelSample>,
    pub environment: Vec<EnvironmentSample>,
}

#[derive(Serialize)]
pub struct SessionSchema {
    #[serde(rename = "initTimestamp")]
    pub init_timestamp: String,
    #[serde(rename = "finishTimestamp")]
    pub finish_timestamp: String,
    #[serde(rename = "dataRaw")]
    pub data_raw: DataRaw,
}

pub struct Storage {
    pub current_dir: PathBuf,
    pub init_ts: String,
    pub data: DataRaw,
    pub reg_count: u32,
}

impl Storage {
    pub fn init() -> Self {
        let base_path = Path::new(CONFIG.storage_path);
        if !base_path.exists() {
            fs::create_dir_all(base_path).expect("Error carpeta base");
        }

        let mut max_index: u32 = 0;
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(s) = name.strip_prefix("register_") {
                        if let Ok(n) = s.parse::<u32>() { if n > max_index { max_index = n; } }
                    }
                }
            }
        }

        let new_path = base_path.join(format!("register_{}", max_index + 1));
        fs::create_dir(&new_path).expect("Error carpeta sesion");

        Self {
            current_dir: new_path,
            init_ts: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            data: DataRaw {
                pressure: Vec::with_capacity(65),
                acceleration: Vec::with_capacity(1250),
                environment: Vec::with_capacity(10),
            },
            reg_count: 1,
        }
    }

    pub fn add_pressure_sample(&mut self, timestamp: String, matrix_ptr: Arc<[[u16; COL_SIZE]; ROW_SIZE]>) {
        self.data.pressure.push(PressureSample { timestamp, measure: matrix_ptr });
    }

    pub fn add_accel_sample(&mut self, timestamp: String, data: [f32; 6]) {
        self.data.acceleration.push(AccelSample { timestamp, measure: data });
    }

    pub fn add_env_sample(&mut self, timestamp: String, temperature: f32, humidity: f32) {
        self.data.environment.push(EnvironmentSample { timestamp, temperature, humidity });
    }

    pub fn flush_chunk(&mut self) {
        let p_len = self.data.pressure.len();
        let a_len = self.data.acceleration.len();
        if p_len == 0 && a_len == 0 { return; }

        let finish_ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let chunk_data = DataRaw {
            pressure: std::mem::take(&mut self.data.pressure),
            acceleration: std::mem::take(&mut self.data.acceleration),
            environment: std::mem::take(&mut self.data.environment),
        };

        let session = SessionSchema {
            init_timestamp: self.init_ts.clone(),
            finish_timestamp: finish_ts.clone(),
            data_raw: chunk_data,
        };

        let file_path = self.current_dir.join(format!("reg_{}.json", self.reg_count));
        let file = File::create(&file_path).unwrap();
        serde_json::to_writer_pretty(file, &session).unwrap();

        println!("[STORAGE] Archivo escrito: reg_{}.json (P: {}, A: {})", self.reg_count, p_len, a_len);

        self.reg_count += 1;
        self.init_ts = finish_ts;
    }
}