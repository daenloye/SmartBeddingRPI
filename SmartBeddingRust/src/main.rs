mod pressure;
mod config;
mod storage;
mod acceleration;
mod environment;
mod audio;
mod api;

use storage::{DataRaw, SessionSchema, AccelSample, PressureSample, Storage, AudioMetrics, Measures};
use pressure::PressureMatrix;
use acceleration::AccelerationModule;
use audio::AudioModule;
use rppal::spi::{Bus, SlaveSelect};
use rppal::i2c::I2c;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use std::io::{self, Write};
use sysinfo::System;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use chrono::Local;

fn logger(module: &str, msg: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] [{}] {}", now, module, msg);
}

#[tokio::main]
async fn main() {
    logger("SISTEMA", "=== Iniciando Estación de Monitoreo Rust ===");

    let storage_dir = Storage::init_path();
    logger("STORAGE", &format!("Directorio de sesión: {}", storage_dir.display()));
    
    let (tx_sensors, mut rx_sensors) = mpsc::channel::<SessionSchema>(10);
    let (tx_audio, mut rx_audio) = mpsc::channel::<AudioMetrics>(10);

    // 1. Audio
    let audio_recorder = AudioModule::new();
    audio_recorder.spawn_recorder(storage_dir.clone(), tx_audio);

    // 2. Worker de Procesamiento y Escritura
    let dir_clone = storage_dir.clone();
    thread::spawn(move || {
        let mut file_count = 1;
        let mut sys_worker = System::new_all();
        while let Some(mut session) = rx_sensors.blocking_recv() {
            // Esperar métricas de audio
            if let Some(metrics) = rx_audio.blocking_recv() {
                session.measures.audio = Some(metrics);
            }

            let path = dir_clone.join(format!("reg_{}.json", file_count));
            Storage::save_session(session, path, &mut sys_worker);
            
            logger("WORKER", &format!("✓ Bloque {} procesado y guardado.", file_count));
            file_count += 1;
        }
    });

    // 3. Hardware
    let shared_i2c = Arc::new(Mutex::new(I2c::new().expect("I2C Fail")));
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init(Arc::clone(&shared_i2c)).expect("Pressure Fail")
    ));

    let p_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = p_hw.write() { s.scan_and_update(); }
            thread::sleep(Duration::from_millis(10));
        }
    });

    // --- NUEVO: INICIAR API ---
    let p_api = Arc::clone(&pressure_sensor);
    let a_api = Arc::clone(&acc_module); // Asegúrate que AccelerationModule sea Arc o clonable

    tokio::spawn(async move {
        api::start_api(p_api, a_api).await;
    });

    // 4. Metrónomo (20Hz -> 50ms)
    let mut ticker = interval(Duration::from_millis(50));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);

    let mut current_data = DataRaw::default();
    let mut init_ts = Local::now().format("%H:%M:%S%.3f").to_string();
    let mut ticks = 0;

    loop {
        ticker.tick().await;
        let ts = Local::now().format("%H:%M:%S%.3f").to_string();
        
        current_data.acceleration.push(AccelSample {
            timestamp: ts.clone(),
            measure: acc_module.get_latest_data(),
        });

        if (ticks + 1) % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                current_data.pressure.push(PressureSample {
                    timestamp: ts.clone(),
                    measure: Arc::new(s.buffers[s.latest_idx]),
                });
            }
            print!("\r[{}] [LIVE] Ticks: {:>4}/1200 | OK", Local::now().format("%H:%M:%S"), ticks + 1);
            io::stdout().flush().ok();
        }

        ticks += 1;

        if ticks >= 1200 { // 60 segundos a 20Hz
            let finish_ts = ts.clone();
            let session = SessionSchema {
                initTimestamp: init_ts.clone(),
                finishTimestamp: finish_ts.clone(),
                dataRaw: std::mem::take(&mut current_data),
                dataProcessed: storage::DataProcessed::default(), // <--- Añade esto
                measures: Measures::default(),
                performance: None,
            };

            let _ = tx_sensors.try_send(session);
            init_ts = finish_ts;
            ticks = 0;
        }
    }
}