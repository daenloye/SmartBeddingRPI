use crate::capture::CaptureController;
use crate::storage::StorageController;
use crate::utils::logger;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct BridgeController {
    // Aquí guardaremos las referencias una vez se inicie el flujo
}

impl BridgeController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&mut self) {
        logger("BRIDGE", "Controlador de puente listo.");
    }

    /// Este es el método que orquestará el paso de datos
    pub fn start(&self, capture: Arc<CaptureController>, storage: Arc<StorageController>) {
        logger("BRIDGE", "Iniciando orquestación de datos...");

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            let tick_rate = Duration::from_millis(10000); // El "Getter" pide cada 1s

            loop {
                logger("BRIDGE", "Obtengo datos...");
                // 1. GETTER: Obtenemos la copia del último dato de ambiente
                let (temp, hum) = capture.environment.get_latest();

                // 2. LÓGICA DE BUFFER (Opcional por ahora):
                // Podrías acumular aquí 10 lecturas antes de llamar al storage
                
                // 3. STORAGE: Enviamos al controlador de almacenamiento
                // storage.save_env_data(temp, hum); 

                // Control del metrónomo del puente
                let elapsed = last_tick.elapsed();
                if elapsed < tick_rate {
                    thread::sleep(tick_rate - elapsed);
                }
                last_tick = Instant::now();
            }
        });
    }
}