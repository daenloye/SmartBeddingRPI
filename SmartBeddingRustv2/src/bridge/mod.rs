use crate::capture::CaptureController;
use crate::storage::StorageController;
use crate::interfaces::{EnvReading, AccelReading}; // Importamos tus nuevas estructuras
use crate::utils::logger;

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use chrono::Local;

pub struct BridgeController {}

impl BridgeController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&mut self) {
        logger("BRIDGE", "Controlador de puente listo.");
    }

    pub fn start(&self, capture: Arc<CaptureController>, storage: Arc<StorageController>) {
        logger("BRIDGE", "Iniciando orquestación de datos...");

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            let mut tick_counter: u32 = 0; 
            let tick_rate = Duration::from_millis(10); 

            loop {
                let timestamp_now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

                // 1. Muestreo de GIROSCOPIO/ACELERACIÓN a 50ms
                if tick_counter % 50 == 0 {
                    let raw = capture.acceleration.get_latest();
                    
                    let reading = AccelReading {
                        gx: raw[0], gy: raw[1], gz: raw[2],
                        ax: raw[3], ay: raw[4], az: raw[5],
                        timestamp: timestamp_now.clone(),
                    };

                    // storage.add_accel(reading); // Enviamos al buffer del storage
                }

                // 2. Muestreo de PRESIÓN a 1000ms (1s)
                if tick_counter % 1000 == 0 {
                    // let press_val = capture.pressure.get_latest();
                    // let reading = PressureReading { ... };
                    // storage.add_pressure(reading);
                }

                // 3. Muestreo de ENVIRONMENT a 10.000ms (10s)
                if tick_counter % 10000 == 0 {
                    logger("BRIDGE", "Capturando snapshot de ambiente...");
                    let env_raw = capture.environment.get_latest();

                    let reading = EnvReading {
                        temperature: env_raw.0,
                        humidity: env_raw.1,
                        timestamp: timestamp_now.clone(),
                    };

                    // storage.add_env(reading);
                }

                // 4. Lógica de CIERRE/ROTACIÓN de archivo (1 minuto)
                if tick_counter >= 60000 {
                    logger("BRIDGE", "Ciclo de 1 minuto completado. Rotando archivos...");
                    // storage.flush_to_disk(); // Orden de escritura pesada
                    tick_counter = 0; 
                } else {
                    tick_counter += 10;
                }

                // 5. Metrónomo de alta precisión (Control de deriva)
                let elapsed = last_tick.elapsed();
                if elapsed < tick_rate {
                    thread::sleep(tick_rate - elapsed);
                }
                last_tick = Instant::now();
            }
        });
    }
}