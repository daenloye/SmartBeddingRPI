// src/storage/audio.rs
use std::path::PathBuf;
use crate::utils::logger;
use hound; 

pub struct AudioHandler;

impl AudioHandler {
    pub fn save_wav(session_path: PathBuf, samples: Vec<i16>) {
        if samples.is_empty() { return; }

        let timestamp = chrono::Local::now().format("%H%M%S").to_string();
        let file_path = session_path.join(format!("audio_{}.wav", timestamp));

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        match hound::WavWriter::create(&file_path, spec) {
            Ok(mut writer) => {
                for sample in samples {
                    let _ = writer.write_sample(sample);
                }
                let _ = writer.finalize();
                logger("AUDIO", &format!("Archivo .wav creado: {:?}", file_path));
            }
            Err(e) => logger("ERROR", &format!("Fallo al crear WAV: {}", e)),
        }
    }
}