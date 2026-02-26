use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use chrono::Local;
use tokio::sync::mpsc as tokio_mpsc;
use crate::storage::AudioMetrics;
use crate::config::CONFIG;

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
            
            // Usamos los valores de CONFIG
            let config = cpal::StreamConfig {
                channels: CONFIG.audio_channels,
                sample_rate: cpal::SampleRate(CONFIG.audio_sample_rate),
                buffer_size: cpal::BufferSize::Default,
            };

            audio_log(&format!("Configurando Hardware: {}Hz, {} canales", 
                CONFIG.audio_sample_rate, CONFIG.audio_channels));

            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _| { let _ = tx.send(data.to_vec()); },
                |err| eprintln!("Audio Error: {}", err),
                None
            ).expect("Error: El hardware no soporta la frecuencia de CONFIG"); 

            stream.play().unwrap();

            while running.load(Ordering::SeqCst) {
                let n = count.fetch_add(1, Ordering::SeqCst);
                let filename = storage_dir.join(format!("audio_{}.wav", n));
                
                if let Ok(mut writer) = hound::WavWriter::create(&filename, hound::WavSpec {
                    channels: CONFIG.audio_channels,
                    sample_rate: CONFIG.audio_sample_rate as u32,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                }) {
                    let mut sum_sq = 0.0;
                    let mut count_samples = 0;
                    let mut max_abs = 0.0f32;
                    let mut zero_crossings = 0;
                    let mut silent_samples = 0;
                    let mut last_s = 0.0f32;

                    let start_block = Instant::now();
                    // Usamos CONFIG para la duración del bloque
                    while start_block.elapsed().as_secs() < CONFIG.audio_block_duration_s && running.load(Ordering::SeqCst) {
                        if let Ok(samples) = rx.recv_timeout(Duration::from_millis(100)) {
                            for s in samples {
                                let abs_s = s.abs();
                                sum_sq += s * s;
                                if abs_s > max_abs { max_abs = abs_s; }
                                if (s > 0.0 && last_s <= 0.0) || (s < 0.0 && last_s >= 0.0) { zero_crossings += 1; }
                                if abs_s < CONFIG.audio_silence_threshold { silent_samples += 1; }
                                last_s = s;
                                count_samples += 1;

                                writer.write_sample((s * i16::MAX as f32) as i16).ok();
                            }
                        }
                    }
                    writer.finalize().ok();

                    let rms = (sum_sq / count_samples.max(1) as f32).sqrt();
                    
                    // 2. CORRECCIÓN: Usar CONFIG para el cálculo del ZCR
                    let metrics = AudioMetrics {
                        db_avg: 20.0 * rms.max(1e-6).log10(),
                        db_max: 20.0 * max_abs.max(1e-6).log10(),
                        db_min: 20.0 * CONFIG.audio_silence_threshold.log10(),
                        zcr: zero_crossings as f32 / (count_samples as f32 / (CONFIG.audio_sample_rate as f32 * CONFIG.audio_channels as f32)),
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