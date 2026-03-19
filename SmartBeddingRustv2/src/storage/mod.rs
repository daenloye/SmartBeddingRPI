use std::fs;
use std::path::{Path, PathBuf};
use crate::utils::logger;

pub struct StorageController {
    pub base_path: PathBuf,
}

impl StorageController {
    pub fn new() -> Self {
        Self {
            // Se define la carpeta base
            base_path: PathBuf::from("SmartBeddingData"),
        }
    }

    pub fn init(&mut self) {
        logger("STORAGE", "Inicializando controladores de almacenamiento...");

        // Creamos la ruta de la sesión actual
        let session_dir = self.create_session_folder();

        // Verificamos o creamos el directorio
        if let Err(e) = fs::create_dir_all(&session_dir) {
            logger("STORAGE", &format!("ERROR CRÍTICO creando carpeta: {}", e));
        } else {
            logger("STORAGE", &format!("Directorio listo en: {:?}", session_dir));
            self.base_path = session_dir;
        }
    }

    fn create_session_folder(&self) -> PathBuf {
        use chrono::Local;
        let now = Local::now().format("%Y%m%d_%H%M%S").to_string();
        
        // Retorna "data/session_TIMESTAMP"
        self.base_path.join(format!("session_{}", now))
    }

    /// Método útil para obtener rutas de archivos dentro de la carpeta de sesión
    pub fn get_file_path(&self, filename: &str) -> PathBuf {
        self.base_path.join(filename)
    }
}