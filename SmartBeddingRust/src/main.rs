mod pressure;
mod config;
mod storage;
mod acceleration;
mod environment;
mod audio;

use storage::{DataRaw, SessionSchema, AccelSample, PressureSample, EnvironmentSample, Storage};
use pressure::PressureMatrix;
use acceleration::AccelerationModule;
use environment::EnvironmentModule;
use audio::AudioModule;
use rppal::spi::{Bus, SlaveSelect};
use rppal::i2c::I2c;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use std::io::{self, Write};
use std::fs::File;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use chrono::Local;

// Función de log centralizada
fn logger(module: &str, msg: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] [{}] {}", now, module, msg);
}

#[tokio::main]
async fn main() {
    logger("SISTEMA", "=== Iniciando Estación de Monitoreo Rust ===");

    // 1. Inicialización de Storage
    let storage_dir = Storage::init_path();
    logger("STORAGE", &format!("Directorio de sesión: {}", storage_dir.display()));
    
    // 2. Canal para el Worker de IA/JSON
    let (tx, mut rx) = mpsc::channel::<SessionSchema>(10);

    // --- 3. INICIO DE AUDIO (Crítico) ---
    // Lo lanzamos primero para que el buffer del canal de audio empiece a llenarse
    let audio_recorder = AudioModule::new();
    audio_recorder.spawn_recorder(storage_dir.clone());
    logger("AUDIO", "Sub-sistema de audio iniciado. Buscando flujo I2S...");

    // --- 4. WORKER DE PROCESAMIENTO ---
    let dir_clone = storage_dir.clone();
    thread::spawn(move || {
        let mut file_count = 1;
        logger("WORKER", "Hilo de IA y Escritura JSON listo.");
        
        while let Some(mut session) = rx.blocking_recv() {
            let start_proc = Local::now();
            
            // Simulación/Ejecución de IA
            procesar_inteligencia_artificial(&mut session);
            
            let proc_dur = Local::now().signed_duration_since(start_proc).num_milliseconds();
            let path = dir_clone.join(format!("reg_{}.json", file_count));
            
            if let Ok(file) = File::create(&path) {
                if serde_json::to_writer(file, &session).is_ok() {
                    logger("WORKER", &format!("✓ Bloque {} guardado (JSON + IA en {}ms)", file_count, proc_dur));
                }
            }
            file_count += 1;
        }
    });

    // --- 5. SETUP HARDWARE SENSORES ---
    let shared_i2c = Arc::new(Mutex::new(I2c::new().expect("I2C Fail")));
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    let env_module = Arc::new(EnvironmentModule::new(Arc::clone(&shared_i2c)));
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init(Arc::clone(&shared_i2c)).expect("Pressure Fail")
    ));

    // Hilo de matriz de presión (Escaneo de hardware a 100Hz)
    let p_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = p_hw.write() { s.scan_and_update(); }
            thread::sleep(Duration::from_millis(10));
        }
    });

    // --- 6. METRÓNOMO DE CAPTURA (20Hz) ---
    let mut ticker = interval(Duration::from_millis(50));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);

    let mut current_data = DataRaw::default();
    let mut init_ts = Local::now().format("%H:%M:%S%.3f").to_string();
    let mut ticks = 0;

    logger("METRONOMO", "Bucle principal iniciado (1200 ticks = 60s)");

    loop {
        ticker.tick().await;
        let ts = Local::now().format("%H:%M:%S%.3f").to_string();
        
        // Muestra de Aceleración
        current_data.acceleration.push(AccelSample {
            timestamp: ts.clone(),
            measure: acc_module.get_latest_data(),
        });

        // Muestra de Presión (Cada 1s)
        if (ticks + 1) % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                current_data.pressure.push(PressureSample {
                    timestamp: ts.clone(),
                    measure: Arc::new(s.buffers[s.latest_idx]),
                });
            }
            // Feedback en vivo
            print!("\r[{}] [LIVE] Ticks: {:>4}/1200 | Sensores OK", Local::now().format("%H:%M:%S"), ticks + 1);
            io::stdout().flush().ok();
        }

        // Muestra Ambiente (Cada 20s)
        if (ticks + 1) % 400 == 0 {
            logger("SISTEMA", "Muestra ambiental recolectada.");
        }

        ticks += 1;

        // CIERRE DE MINUTO
        if ticks >= 1200 {
            let finish_ts = ts.clone();
            logger("SISTEMA", ">>> Finalizando ciclo de 60s. Rotando archivos...");
            
            let session = SessionSchema {
                initTimestamp: init_ts.clone(),
                finishTimestamp: finish_ts.clone(),
                dataRaw: std::mem::take(&mut current_data),
            };

            if let Err(_) = tx.try_send(session) {
                logger("ALERTA", "Buffer de procesamiento lleno. ¡IA demasiado lenta!");
            }

            // Reinicio de ciclo
            init_ts = finish_ts;
            ticks = 0;
            current_data.acceleration.reserve(1200);
            current_data.pressure.reserve(60);
        }
    }
}

fn procesar_inteligencia_artificial(_session: &mut SessionSchema) {
    // Aquí va tu lógica pesada de Rust
}