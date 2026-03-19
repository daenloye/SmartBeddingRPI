use crate::capture::CaptureController;
use crate::storage::{StorageController};
use crate::interfaces::{EnvReading, AccelReading, PressureReading, DataRaw}; 
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
        thread::spawn(move || {
            let tick_rate = Duration::from_millis(10); // Resolución base de 10ms
            let mut next_tick = Instant::now();

            let mut tick_counter: u32 = 0;
            let mut current_data = DataRaw::default();
            let mut minute_start_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            loop {
                let timestamp_now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

                // --- LÓGICA DE MUESTREO EXACTO ---

                // 1. ACELERACIÓN: 50ms (Exactos)
                if tick_counter % 50 == 0 {
                    let raw = capture.acceleration.get_latest();
                    current_data.acceleration.push(AccelReading {
                        gx: raw[0], gy: raw[1], gz: raw[2],
                        ax: raw[3], ay: raw[4], az: raw[5],
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 2. PRESIÓN: 1000ms (1s Exacto)
                if tick_counter % 1000 == 0 {
                    let matrix = capture.pressure.get_latest();
                    current_data.pressure.push(PressureReading {
                        matrix,
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 3. AMBIENTE: 20.000ms (20s Exactos)
                if tick_counter % 20000 == 0 {
                    let env = capture.environment.get_latest();
                    current_data.environment.push(EnvReading {
                        temperature: env.0,
                        humidity: env.1,
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 4. ROTACIÓN DE MINUTO: 60.000ms
                if tick_counter >= 60000 {
                    let data_to_save = std::mem::take(&mut current_data);
                    let start_to_save = minute_start_time.clone();
                    let storage_ptr = storage.clone();

                    thread::spawn(move || {
                        storage_ptr.process_and_save(data_to_save, start_to_save);
                    });

                    tick_counter = 0;
                    minute_start_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                } else {
                    tick_counter += 10;
                }

                // --- EL METRÓNOMO ABSOLUTO ---
                next_tick += tick_rate; // Calculamos cuándo DEBERÍA ser el siguiente tick
                let now = Instant::now();
                
                if next_tick > now {
                    thread::sleep(next_tick - now); // Dormimos solo el tiempo restante
                } else {
                    // Si llegamos tarde (la CPU se saturó), no dormimos 
                    // y recalculamos next_tick para no acumular error
                    next_tick = now;
                }
            }
        });
    }
}