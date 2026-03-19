use std::path::PathBuf;
use crate::utils::logger;
use crate::interfaces::AudioMetrics;
use hound;

pub struct AudioHandler;

impl AudioHandler {
    /// Analiza el buffer para extraer métricas de decibelios
    pub fn analyze_buffer(samples: &[i16]) -> AudioMetrics {
        if samples.is_empty() {
            return AudioMetrics { db_avg: -90.0, db_max: -90.0, db_min: -90.0 };
        }

        let mut sum_sq = 0.0f64;
        let mut max_abs = 0;
        let mut min_abs = i16::MAX;

        for &s in samples {
            let abs_s = s.abs();
            sum_sq += (s as f64 * s as f64);
            if abs_s > max_abs { max_abs = abs_s; }
            if abs_s < min_abs && abs_s > 0 { min_abs = abs_s; }
        }

        let rms = (sum_sq / samples.len() as f64).sqrt();
        
        // Referencia para i16 (32767 es 0 dBFS)
        let to_db = |v: f64| {
            let db = 20.0 * (v / 32767.0).max(1e-5).log10();
            db as f32
        };

        AudioMetrics {
            db_avg: to_db(rms),
            db_max: to_db(max_abs as f64),
            db_min: to_db(min_abs as f64),
        }
    }

    pub fn save_wav(file_path: PathBuf, samples: Vec<i16>) {
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
            }
            Err(e) => logger("ERROR", &format!("Fallo al crear WAV: {}", e)),
        }
    }
}