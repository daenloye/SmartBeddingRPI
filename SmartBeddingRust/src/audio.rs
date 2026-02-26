use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use chrono::Local;
use tokio::sync::mpsc as tokio_mpsc;
use crate::storage::AudioMetrics;

fn audio_log(msg: &str) {
    let now = Local::now().format("%H:%M:%S");
    println!("[{}] [AUDIO_STATUS] {}", now, msg);
}

pub struct AudioModule {
    running: Arc<AtomicBool>,
    file_count: Arc<AtomicU32>,
}

impl AudioModule {
    pub fn new() -> Self {
        Self { 
            running: Arc::new(AtomicBool::new(true)),
            file_count: Arc::new(AtomicU32::new(1)),
        }
    }

    pub fn spawn_recorder(&self, storage_dir: PathBuf, metrics_tx: tokio_mpsc::Sender<AudioMetrics>) {
        let running = Arc::clone(&self.running);
        let count = Arc::clone(&self.file_count);
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(500);

        std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = host.default_input_device().expect("No I2S device");
            let config = device.default_input_config().expect("Config error");
            let sample_rate = config.sample_rate().0 as f32;
            let channels = config.channels() as f32;

            audio_log(&format!("Hardware listo: {}Hz, {} canales", sample_rate, channels));

            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| { let _ = tx.send(data.to_vec()); },
                |err| eprintln!("Audio Error: {}", err),
                None
            ).unwrap();

            stream.play().unwrap();

            while running.load(Ordering::SeqCst) {
                let n = count.fetch_add(1, Ordering::SeqCst);
                let filename = storage_dir.join(format!("audio_{}.wav", n));
                
                if let Ok(mut writer) = hound::WavWriter::create(&filename, hound::WavSpec {
                    channels: channels as u16,
                    sample_rate: sample_rate as u32,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                }) {
                    let mut sum_sq = 0.0;
                    let mut count_samples = 0;
                    let mut max_abs = 0.0f32;
                    let mut zero_crossings = 0;
                    let mut silent_samples = 0;
                    let mut last_s = 0.0f32;
                    let silence_threshold = 0.01;

                    let start_block = Instant::now();
                    while start_block.elapsed().as_secs() < 60 && running.load(Ordering::SeqCst) {
                        if let Ok(samples) = rx.recv_timeout(Duration::from_millis(100)) {
                            for s in samples {
                                let abs_s = s.abs();
                                sum_sq += s * s;
                                if abs_s > max_abs { max_abs = abs_s; }
                                if (s > 0.0 && last_s <= 0.0) || (s < 0.0 && last_s >= 0.0) { zero_crossings += 1; }
                                if abs_s < silence_threshold { silent_samples += 1; }
                                last_s = s;
                                count_samples += 1;

                                writer.write_sample((s * i16::MAX as f32) as i16).ok();
                            }
                        }
                    }
                    writer.finalize().ok();

                    let rms = (sum_sq / count_samples.max(1) as f32).sqrt();
                    let metrics = AudioMetrics {
                        db_avg: 20.0 * rms.max(1e-6).log10(),
                        db_max: 20.0 * max_abs.max(1e-6).log10(),
                        db_min: 20.0 * silence_threshold.log10(),
                        zcr: zero_crossings as f32 / (count_samples as f32 / (sample_rate * channels)),
                        crest_factor: if rms > 0.0 { max_abs / rms } else { 0.0 },
                        silence_percent: (silent_samples as f32 / count_samples.max(1) as f32) * 100.0,
                    };
                    
                    let _ = metrics_tx.blocking_send(metrics);
                    audio_log(&format!("✓ Bloque {} guardado y analizado.", n));
                }
            }
        });
    }
}