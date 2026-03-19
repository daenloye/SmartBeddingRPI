use crate::capture::CaptureController;
use crate::storage::StorageController;
use crate::interfaces::{EnvReading, AccelReading, PressureReading, DataRaw}; 
use crate::utils::logger;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use chrono::Local;

pub struct BridgeController {}

impl BridgeController {
    pub fn new() -> Self { Self {} }

    pub fn init(&mut self) { 
        logger("BRIDGE", "Controlador con aislamiento de audio listo."); 
    }

    pub fn start(&self, capture: Arc<CaptureController>, storage: Arc<StorageController>) {
        // --- ESTADO COMPARTIDO PARA AUDIO ---
        // Este buffer es el que Bridge llena desde un hilo y vacía desde el otro
        let bridge_audio_buffer = Arc::new(Mutex::new(Vec::with_capacity(44100 * 60)));
        
        let audio_capture_ptr = capture.clone();
        let audio_buffer_ptr = Arc::clone(&bridge_audio_buffer);

        // --- HILO 1: EXTRACTOR DE AUDIO (Alta Prioridad) ---
        thread::spawn(move || {
            let tick_audio = Duration::from_millis(10);
            loop {
                let start = Instant::now();
                
                // Succionamos samples del módulo de captura al buffer del Bridge
                let samples = audio_capture_ptr.audio.pull_samples();
                if !samples.is_empty() {
                    if let Ok(mut lock) = audio_buffer_ptr.lock() {
                        lock.extend(samples);
                    }
                }

                // Dormimos lo justo para mantener el ritmo de 10ms
                let elapsed = start.elapsed();
                if tick_audio > elapsed {
                    thread::sleep(tick_audio - elapsed);
                }
            }
        });

        // --- HILO 2: SENSORES Y ORQUESTACIÓN (El Metrónomo) ---
        thread::spawn(move || {
            let tick_rate = Duration::from_millis(10);
            let mut next_tick = Instant::now();
            let mut tick_counter: u32 = 0;
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

                // 2. PRESIÓN (1s)
                if tick_counter % 1000 == 0 {
                    let matrix = capture.pressure.get_latest();
                    current_data.pressure.push(PressureReading {
                        matrix,
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 3. AMBIENTE (20s)
                if tick_counter % 20000 == 0 {
                    let env = capture.environment.get_latest();
                    current_data.environment.push(EnvReading {
                        temperature: env.0, humidity: env.1,
                        timestamp: timestamp_now.clone(),
                    });
                }

                // 4. ROTACIÓN DE MINUTO (60s)
                if tick_counter >= 60000 {
                    let data_to_save = std::mem::take(&mut current_data);
                    let start_to_save = minute_start_time.clone();
                    
                    // EXTRAEMOS EL AUDIO DEL BUFFER INTERNO DEL BRIDGE
                    let audio_to_save = if let Ok(mut lock) = bridge_audio_buffer.lock() {
                        std::mem::take(&mut *lock)
                    } else { Vec::new() };

                    let storage_ptr = storage.clone();
                    thread::spawn(move || {
                        storage_ptr.process_and_save(data_to_save, audio_to_save, start_to_save);
                    });

                    tick_counter = 0;
                    minute_start_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                } else {
                    tick_counter += 10;
                }

                // METRÓNOMO
                next_tick += tick_rate;
                let now = Instant::now();
                if next_tick > now { thread::sleep(next_tick - now); } else { next_tick = now; }
            }
        });
    }
}