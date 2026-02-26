use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant}; // Usaremos Instant para el tiempo real
use chrono::Local;

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

    pub fn spawn_recorder(&self, storage_dir: PathBuf) {
        let running = Arc::clone(&self.running);
        let count = Arc::clone(&self.file_count);
        // Canal para recibir muestras F32 (que es lo que tu micro escupe)
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(500);

        std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = host.default_input_device().expect("No I2S device");
            let config = device.default_input_config().expect("Config error");
            let sample_rate = config.sample_rate().0;
            let channels = config.channels() as u16;

            audio_log(&format!("Hardware listo: {}Hz, {} canales", sample_rate, channels));

            // Stream para F32
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
                
                let spec = hound::WavSpec {
                    channels,
                    sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };

                if let Ok(mut writer) = hound::WavWriter::create(&filename, spec) {
                    audio_log(&format!(">>> Grabando bloque: audio_{}.wav", n));
                    
                    let start_block = Instant::now();
                    
                    // GRABAR POR TIEMPO (60 segundos exactos)
                    while start_block.elapsed().as_secs() < 60 && running.load(Ordering::SeqCst) {
                        if let Ok(samples) = rx.recv_timeout(Duration::from_millis(100)) {
                            for s in samples {
                                // Conversión F32 -> I16
                                let sample_i16 = (s * i16::MAX as f32) as i16;
                                writer.write_sample(sample_i16).ok();
                            }
                        }
                    }
                    writer.finalize().ok();
                    audio_log(&format!("✓ Bloque {} guardado (60s reales).", n));
                }
            }
        });
    }
}