use crate::utils::logger;
use std::sync::{Arc, Mutex};

pub struct AudioModule {
    // Buffer intermedio para acumular muestras entre ticks del Bridge
    internal_queue: Arc<Mutex<Vec<i16>>>,
}

impl AudioModule {
    pub fn new() -> Self {
        Self {
            internal_queue: Arc::new(Mutex::new(Vec::with_capacity(44100))),
        }
    }

    pub fn init(&self) {
        logger("AUDIO", "Configurando dispositivo de entrada de audio...");
        // Aquí configurarías CPAL/ALSA en el futuro
    }

    pub fn start(&self) {
        logger("AUDIO", "Stream de audio iniciado.");
        // Aquí lanzarías el hilo que llena el buffer
    }

    /// El Bridge llama aquí en cada tick (10ms) para vaciar lo acumulado
    pub fn pull_samples(&self) -> Vec<i16> {
        let mut lock = self.internal_queue.lock().unwrap();
        std::mem::take(&mut *lock)
    }

    /// Método interno para que el callback del driver inserte datos
    pub fn push_samples(&self, samples: &[i16]) {
        if let Ok(mut lock) = self.internal_queue.lock() {
            lock.extend_from_slice(samples);
        }
    }
}