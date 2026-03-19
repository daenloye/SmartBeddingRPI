pub mod files;
pub mod audio;

use files::FileHandler;
use crate::interfaces::DataRaw;
use crate::utils::logger;

pub struct StorageController {
    file_handler: Option<FileHandler>,
}

impl StorageController {
    pub fn new() -> Self {
        Self { file_handler: None }
    }

    pub fn init(&mut self) {
        self.file_handler = Some(FileHandler::new());
        logger("STORAGE", "Orquestador iniciado.");
    }

    /// Pasamanos puro
    pub fn process_and_save(&self, raw_data: DataRaw, start_time: String) {
        if let Some(handler) = &self.file_handler {
            handler.process_and_persist(raw_data, start_time);
        }
    }
}