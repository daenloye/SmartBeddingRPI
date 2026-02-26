use std::fs;
use std::path::{Path, PathBuf};
use crate::config::CONFIG;

pub struct Storage {
    pub current_dir: PathBuf,
}

impl Storage {
    pub fn init() -> Self {
        // 1. Aseguramos que el path base exista
        let base_path = Path::new(CONFIG.storage_path);
        if !base_path.exists() {
            fs::create_dir_all(base_path).expect("[STORAGE] No se pudo crear la carpeta base");
        }

        // 2. Contar carpetas existentes (max_index debe ser u32 explícito)
        let mut max_index: u32 = 0;
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Corrección del error E0599: file_name() -> Option<&OsStr> -> to_str()
                    if let Some(os_name) = path.file_name() {
                        if let Some(name_str) = os_name.to_str() {
                            if let Some(stripped) = name_str.strip_prefix("register_") {
                                // Corrección del error E0282: parse ya sabe que es u32 por max_index
                                if let Ok(n) = stripped.parse::<u32>() {
                                    if n > max_index {
                                        max_index = n;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Crear la nueva carpeta register_n+1
        let new_folder_name = format!("register_{}", max_index + 1);
        let new_path = base_path.join(new_folder_name);

        fs::create_dir(&new_path).expect("[STORAGE] No se pudo crear la nueva carpeta de registro");

        if CONFIG.debug_mode {
            println!("[STORAGE] Sesión iniciada en: {:?}", new_path);
        }

        Self {
            current_dir: new_path,
        }
    }
}