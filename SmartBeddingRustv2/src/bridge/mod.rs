use crate::capture::CaptureController;
use crate::storage::{StorageController, DataRaw};
use crate::interfaces::{EnvReading, AccelReading, PressureReading}; 
use crate::utils::logger;

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use chrono::Local;

pub struct BridgeController {}

impl BridgeController {
    pub fn new() -> Self { Self {} }

    pub fn init(&mut self) { logger("BRIDGE", "Controlador listo."); }

    pub fn start(&self, capture: Arc<CaptureController>, storage: Arc<StorageController>) {
        logger("BRIDGE", "Iniciando orquestación...");

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            let mut tick_counter: u32 = 0; 
            let tick_rate = Duration::from_millis(10); 

            // Buffers locales al hilo del Bridge
            let mut current_data = DataRaw::default();
            let mut minute_start_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            loop {
                let timestamp_now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

                // 1. ACELERACIÓN (50ms)
                if tick_counter % 50 == 0 {
                    let raw = capture.acceleration.get_latest();
                    current_data.acceleration.push(AccelReading {
                        gx: raw[0], gy: raw[1], gz: raw[2],
                        ax: raw[3], ay: raw[4], az: raw[5],
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 2. PRESIÓN (1000ms)
                if tick_counter % 1000 == 0 {
                    let matrix = capture.pressure.get_latest();
                    current_data.pressure.push(PressureReading {
                        matrix,
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 3. AMBIENTE (10s)
                if tick_counter % 10000 == 0 {
                    let env = capture.environment.get_latest();
                    current_data.environment.push(EnvReading {
                        temperature: env.0,
                        humidity: env.1,
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 4. ROTACIÓN DE MINUTO
                if tick_counter >= 60000 {
                    logger("BRIDGE", "--- Rotando minuto ---");
                    
                    // Extraer datos y resetear buffer instantáneamente
                    let data_to_save = std::mem::take(&mut current_data);
                    let start_to_save = minute_start_time.clone();
                    let storage_ptr = storage.clone();

                    // Guardado asíncrono para no bloquear el siguiente tick
                    thread::spawn(move || {
                        storage_ptr.process_and_save(data_to_save, start_to_save);
                    });

                    tick_counter = 0;
                    minute_start_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                } else {
                    tick_counter += 10;
                }

                // Metrónomo
                let elapsed = last_tick.elapsed();
                if elapsed < tick_rate { thread::sleep(tick_rate - elapsed); }
                last_tick = Instant::now();
            }
        });
    }
}