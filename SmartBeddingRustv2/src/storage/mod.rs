pub mod files;
pub mod audio;

use files::FileHandler;
use audio::AudioHandler;
use crate::interfaces::DataRaw;
use crate::utils::logger;
use std::sync::Arc;

pub struct StorageController {
    file_handler: Arc<FileHandler>,
}

impl StorageController {
    pub fn new() -> Self {
        Self {
            file_handler: Arc::new(FileHandler::new()),
        }
    }

    pub fn init(&mut self) {
        logger("STORAGE", "Orquestador de archivos listo.");
    }

    pub fn process_and_save(&self, mut raw_data: DataRaw, audio_data: Vec<i16>, start_time: String) {
        let handler = self.file_handler.clone();
        
        // 1. DSP: Analizar audio primero para incluirlo en el JSON
        let metrics = AudioHandler::analyze_buffer(&audio_data);
        raw_data.audio_summary = Some(metrics);

        // 2. Persistencia JSON (Sensores + Resumen Audio)
        let session_path = handler.session_path.clone();
        handler.process_and_persist(raw_data, start_time);

        // 3. Persistencia WAV (Audio crudo)
        let timestamp = chrono::Local::now().format("%H%M%S").to_string();
        let wav_path = session_path.join(format!("audio_{}.wav", timestamp));
        AudioHandler::save_wav(wav_path, audio_data);
        
        logger("STORAGE", "Ciclo de guardado de minuto completado.");
    }
}