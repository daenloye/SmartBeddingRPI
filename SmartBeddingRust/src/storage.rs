use std::fs;
use std::path::{Path, PathBuf};
use crate::config::CONFIG;
use crate::pressure::{COL_SIZE, ROW_SIZE};

// Estructura para representar una sola muestra de la cama
pub struct PressureSample {
    pub timestamp: String,
    pub matrix: [[u16; COL_SIZE]; ROW_SIZE],
}

pub struct Storage {
    pub current_dir: PathBuf,
    // Buffer en memoria para almacenar las muestras
    pub pressure_buffer: Vec<PressureSample>,
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
            println!("[STORAGE] Sesión iniciada y buffer listo en memoria.");
        }

        Self {
            current_dir: new_path,
            pressure_buffer: Vec::with_capacity(100), // Iniciamos con capacidad para 100 muestras para evitar realocaciones constantes
        }
    }

    /// Recibe la matriz y el timestamp y los guarda en el vector de memoria
    pub fn add_sample(&mut self, timestamp: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
        let sample = PressureSample {
            timestamp,
            matrix,
        };
        self.pressure_buffer.push(sample);

        if CONFIG.debug_mode {
            println!("[STORAGE] Muestra añadida al buffer. Total: {}", self.pressure_buffer.len());
        }
    }
}