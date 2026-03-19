use std::path::PathBuf;
use crate::utils::logger;
use crate::interfaces::AudioMeasures; // Nombre actualizado según interfaces
use hound;

pub struct AudioHandler;

impl AudioHandler {
    /// Analiza el buffer para extraer métricas detalladas (dBs, ZCR, Crest Factor)
    pub fn analyze_buffer(samples: &[i16]) -> AudioMeasures {
        if samples.is_empty() {
            return AudioMeasures::default();
        }

        let mut sum_sq = 0.0f64;
        let mut max_abs = 0i16;
        let mut zero_crossings = 0;
        let mut silence_count = 0;
        let silence_threshold = 50; // Umbral para considerar silencio

        for i in 0..samples.len() {
            let s = samples[i];
            let abs_s = s.abs();
            
            // Eliminamos paréntesis innecesarios para evitar warnings
            sum_sq += s as f64 * s as f64;
            
            if abs_s > max_abs { max_abs = abs_s; }
            if abs_s < silence_threshold { silence_count += 1; }

            // Zero Crossing Rate (ZCR)
            if i > 0 && ((samples[i-1] >= 0 && s < 0) || (samples[i-1] < 0 && s >= 0)) {
                zero_crossings += 1;
            }
        }

        let total_samples = samples.len() as f64;
        let rms = (sum_sq / total_samples).sqrt();
        
        // Función para convertir a dBFS (Referencia 32767)
        let to_db = |v: f64| (20.0 * (v / 32767.0).max(1e-5).log10()) as f32;

        let db_avg = to_db(rms);
        let crest_factor = if rms > 0.0 { (max_abs as f64 / rms) as f32 } else { 0.0 };

        AudioMeasures {
            db_avg,
            db_max: to_db(max_abs as f64),
            db_min: -40.0, // Valor de referencia mínimo
            zcr: (zero_crossings as f32 / total_samples as f32),
            crest_factor,
            silence_percent: (silence_count as f32 / total_samples as f32) * 100.0,
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