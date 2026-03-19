use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::Write;
use crate::utils::logger;
use crate::interfaces::AudioMeasures;
use hound;

pub struct AudioHandler;

impl AudioHandler {
    pub fn analyze_buffer(samples: &[i16]) -> AudioMeasures {
        if samples.is_empty() { return AudioMeasures::default(); }

        let mut sum_sq = 0.0f64;
        let mut max_abs = 0i16;
        let mut zero_crossings = 0;
        let mut silence_count = 0;
        let silence_threshold = 50; 

        for i in 0..samples.len() {
            let s = samples[i];
            let abs_s = s.abs();
            sum_sq += s as f64 * s as f64;
            if abs_s > max_abs { max_abs = abs_s; }
            if abs_s < silence_threshold { silence_count += 1; }
            if i > 0 && ((samples[i-1] >= 0 && s < 0) || (samples[i-1] < 0 && s >= 0)) {
                zero_crossings += 1;
            }
        }

        let total_samples = samples.len() as f64;
        let rms = (sum_sq / total_samples).sqrt();
        let to_db = |v: f64| (20.0 * (v / 32767.0).max(1e-5).log10()) as f32;

        AudioMeasures {
            db_avg: to_db(rms),
            db_max: to_db(max_abs as f64),
            db_min: -40.0,
            zcr: (zero_crossings as f32 / total_samples as f32),
            crest_factor: if rms > 0.0 { (max_abs as f64 / rms) as f32 } else { 0.0 },
            silence_percent: (silence_count as f32 / total_samples as f32) * 100.0,
        }
    }

    /// MÉTODO MAESTRO: Guarda en WAV, OPUS o AMBOS usando FFmpeg
    pub fn save_audio(
        base_path: PathBuf, 
        samples: &[i16], 
        do_wav: bool, 
        do_opus: bool
    ) {
        if !do_wav && !do_opus { return; }

        // Si solo es WAV y no queremos FFmpeg, podemos seguir usando hound (más ligero)
        if do_wav && !do_opus {
            let wav_path = base_path.with_extension("wav");
            Self::save_wav_native(wav_path, samples);
            return;
        }

        // Si hay OPUS de por medio, invocamos a FFmpeg
        let mut args = vec![
            "-y".to_string(),
            "-f".to_string(), "s16le".to_string(),
            "-ar".to_string(), "44100".to_string(),
            "-ac".to_string(), "1".to_string(),
            "-i".to_string(), "pipe:0".to_string(),
        ];

        if do_opus {
            let opus_path = base_path.with_extension("opus");
            args.extend(["-c:a:0".to_string(), "libopus".to_string(), 
                         "-b:a:0".to_string(), "32k".to_string(), 
                         opus_path.to_str().unwrap().to_string()]);
        }

        if do_wav {
            let wav_path = base_path.with_extension("wav");
            args.extend(["-c:a:1".to_string(), "pcm_s16le".to_string(), 
                         wav_path.to_str().unwrap().to_string()]);
        }

        let mut child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Fallo al iniciar FFmpeg");

        if let Some(mut stdin) = child.stdin.take() {
            // Convertimos i16 a bytes (le) para el pipe
            let mut byte_buffer = Vec::with_capacity(samples.len() * 2);
            for &sample in samples {
                byte_buffer.extend_from_slice(&sample.to_le_bytes());
            }
            let _ = stdin.write_all(&byte_buffer);
        }
        let _ = child.wait();
    }

    fn save_wav_native(path: PathBuf, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int,
        };
        if let Ok(mut writer) = hound::WavWriter::create(path, spec) {
            for &s in samples { let _ = writer.write_sample(s); }
        }
    }
}