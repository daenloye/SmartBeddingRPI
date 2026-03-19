// src/storage/mod.rs

pub mod files;
pub mod audio;

use files::FileHandler;
use audio::AudioHandler;
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
        logger("STORAGE", "Orquestador listo.");
    }

    pub fn process_and_save(&self, raw_data: DataRaw, audio_data: Vec<i16>, start_time: String) {
        if let Some(handler) = &self.file_handler {
            // Guardamos el JSON y procesamos señales (files.rs)
            handler.process_and_persist(raw_data, start_time);
            
            // Guardamos el audio (audio.rs)
            AudioHandler::save_wav(handler.session_path.clone(), audio_data);
        }
    }
}