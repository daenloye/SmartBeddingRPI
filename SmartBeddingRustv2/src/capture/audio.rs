use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::utils::logger;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

pub struct AudioModule {
    // Cola para el Bridge
    internal_queue: Arc<Mutex<Vec<i16>>>,
    // Control de estado
    running: Arc<AtomicBool>,
}

impl AudioModule {
    pub fn new() -> Self {
        Self {
            internal_queue: Arc::new(Mutex::new(Vec::with_capacity(44100))),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn init(&self) {
        logger("AUDIO", "Buscando dispositivo de entrada predeterminado...");
        let host = cpal::default_host();
        let _device = host.default_input_device()
            .expect("Error crítico: No se encontró dispositivo de entrada de audio");
        logger("AUDIO", "Hardware detectado correctamente.");
    }

    pub fn start(&self) {
        if self.running.load(Ordering::SeqCst) { return; }
        self.running.store(true, Ordering::SeqCst);

        let running = Arc::clone(&self.running);
        let queue = Arc::clone(&self.internal_queue);

        std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = host.default_input_device().expect("Fallo al obtener input device");
            
            // Configuración estándar: Mono, 44.1kHz (Ajustar según CONFIG si es necesario)
            let config = cpal::StreamConfig {
                channels: 1, 
                sample_rate: cpal::SampleRate(44100),
                buffer_size: cpal::BufferSize::Default,
            };

            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    if let Ok(mut lock) = queue.lock() {
                        // Convertimos f32 (-1.0 a 1.0) a i16 para el Bridge
                        for &sample in data {
                            let s = (sample * i16::MAX as f32) as i16;
                            lock.push(s);
                        }
                    }
                },
                |err| eprintln!("Audio Stream Error: {}", err),
                None
            ).expect("No se pudo construir el stream de audio");

            stream.play().expect("No se pudo iniciar el stream");
            logger("AUDIO", "Captura de hardware activa.");

            while running.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
    }

    /// El Bridge llama aquí cada 10ms
    pub fn pull_samples(&self) -> Vec<i16> {
        let mut lock = self.internal_queue.lock().unwrap();
        std::mem::take(&mut *lock)
    }
}