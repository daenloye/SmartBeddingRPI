use crate::pressure::{COL_SIZE, ROW_SIZE};
use crate::config::CONFIG;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct PressureSnapshot {
    pub timestamp: String,
    pub matrix: [[u16; COL_SIZE]; ROW_SIZE],
}

pub struct StorageSystem {
    session_path: PathBuf,
    pub pressure_buffer: Vec<PressureSnapshot>,
}

impl StorageSystem {
    pub fn new() -> Self {
        let base_path = CONFIG.storage_path;
        let _ = fs::create_dir_all(base_path);

        let n = fs::read_dir(base_path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter(|e| e.file_name().to_string_lossy().starts_with("register_"))
                    .count()
            })
            .unwrap_or(0);

        let session_path = PathBuf::from(base_path).join(format!("register_{}", n + 1));
        
        if CONFIG.storage_enabled {
            let _ = fs::create_dir_all(&session_path).ok();
        }

        Self {
            session_path,
            pressure_buffer: Vec::with_capacity(200),
        }
    }

    pub fn add_pressure_snapshot(&mut self, snapshot: PressureSnapshot) {
        self.pressure_buffer.push(snapshot);
    }

    pub fn trigger_flush(&mut self) {
        let count = self.pressure_buffer.len();
        if count == 0 {
            if CONFIG.debug_mode { println!("[STORAGE] Nada que guardar."); }
            return;
        }

        println!("\n[TRIGGER] >>> Almacenando {} muestras en {:?}...", count, self.session_path);
        
        // Aquí podrías implementar el guardado real a CSV si lo deseas
        self.pressure_buffer.clear();
        
        println!("[STORAGE] >>> Buffer limpio.\n");
    }
}