pub mod files;
pub mod audio;

use files::FileHandler;
use crate::interfaces::{DataRaw, SessionSchema, Measures, DataProcessed}; 
use crate::utils::logger;
use std::sync::Arc;

pub struct StorageController {
    // Definido como Arc directo, sin Option
    pub file_handler: Arc<FileHandler>,
}

impl StorageController {
    pub fn new() -> Self {
        Self {
            file_handler: Arc::new(FileHandler::new()),
        }
    }

    pub fn init(&mut self) {
        logger("STORAGE", "Orquestador listo.");
    }

    pub fn process_and_save(&self, raw_data: DataRaw, audio_data: Vec<i16>, start_time: String) {
        // --- LA CORRECCIÓN ESTÁ AQUÍ ---
        // Eliminamos el 'if let Some' porque self.file_handler NO es un Option.
        // Se accede directamente:
        self.file_handler.process_and_persist(raw_data, audio_data, start_time);
    }
}