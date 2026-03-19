use std::path::PathBuf;
use crate::utils::logger;
use hound; // Asegúrate de tener 'hound' en el Cargo.toml

pub struct AudioHandler;

impl AudioHandler {
    pub fn save_wav(session_path: PathBuf, samples: Vec<i16>) {
        if samples.is_empty() {
            logger("AUDIO", "Buffer vacío, saltando guardado.");
            return;
        }

        let timestamp = chrono::Local::now().format("%H%M%S").to_string();
        let file_path = session_path.join(format!("audio_{}.wav", timestamp));

        // Configuración para WAV estándar (Mono, 44.1kHz, 16-bit)
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
                writer.finalize().expect("Error al cerrar el archivo WAV");
                logger("AUDIO", &format!("✓ Grabación guardada: {:?}", file_path));
            }
            Err(e) => logger("ERROR", &format!("No se pudo crear el WAV: {}", e)),
        }
    }
}