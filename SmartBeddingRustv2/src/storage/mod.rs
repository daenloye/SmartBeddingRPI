pub mod files;
pub mod audio;

use files::FileHandler;
use crate::interfaces::*;
use crate::utils::logger;
use sysinfo::{System, SystemExt, CpuExt};
use chrono::Local;

pub struct StorageController {
    file_handler: Option<FileHandler>,
}

impl StorageController {
    pub fn new() -> Self {
        Self { file_handler: None }
    }

    pub fn init(&mut self) {
        self.file_handler = Some(FileHandler::new());
        logger("STORAGE", "Controlador de almacenamiento listo.");
    }

    pub fn process_and_save(&self, raw_data: DataRaw, start_time: String) {
        let handler = self.file_handler.as_ref().expect("FileHandler no inicializado");

        // 1. DSP (Delegado a files.rs)
        let (rrs, crs, resp_rate) = handler.run_dsp(&raw_data);

        // 2. Performance
        let mut sys = System::new_all();
        sys.refresh_all();
        let performance = Performance {
            cpu_percent: sys.global_cpu_info().cpu_usage(),
            mem_percent: (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
        };

        // 3. Schema
        let schema = SessionSchema {
            initTimestamp: start_time,
            finishTimestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            dataRaw: raw_data, 
            dataProcessed: DataProcessed { rrs, crs },
            measures: Measures { respiratory_rate: resp_rate, ..Default::default() },
            performance: Some(performance),
        };

        // 4. Guardar
        handler.save_session_json(&schema);
    }
}