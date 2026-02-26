mod pressure;
mod config;
mod storage;
mod acceleration;
mod environment;

use storage::{DataRaw, SessionSchema, AccelSample, PressureSample, EnvironmentSample, Storage};
use pressure::PressureMatrix;
use acceleration::AccelerationModule;
use environment::EnvironmentModule;
use rppal::spi::{Bus, SlaveSelect};
use rppal::i2c::I2c;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use std::io::{self, Write};
use std::fs::File;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use tokio::signal;
use chrono::Local;

#[tokio::main]
async fn main() {
    let storage_dir = Storage::init_path();
    let (file_tx, mut file_rx) = mpsc::channel::<SessionSchema>(15);

    // HILO DE DISCO (Escritura background)
    let dir_clone = storage_dir.clone();
    tokio::spawn(async move {
        let mut reg_count = 1;
        while let Some(session) = file_rx.recv().await {
            let path = dir_clone.join(format!("reg_{}.json", reg_count));
            if let Ok(file) = File::create(&path) {
                let _ = serde_json::to_writer(file, &session);
            }
            reg_count += 1;
        }
    });

    let shared_i2c = Arc::new(Mutex::new(I2c::new().expect("I2C Fail")));
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    let env_module = Arc::new(EnvironmentModule::new(Arc::clone(&shared_i2c)));
    let pressure_sensor: Arc<RwLock<PressureMatrix>> = Arc::new(RwLock::new(
        PressureMatrix::init(Arc::clone(&shared_i2c)).expect("Pressure Fail")
    ));

    // El ticker manda: 20Hz = 50ms por tick. 1200 ticks = 60 segundos.
    let mut ticker = interval(Duration::from_millis(50));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);

    let mut current_data = DataRaw::default();
    let mut init_ts = Local::now().format("%H:%M:%S%.3f").to_string();
    let mut ticks = 0;

    println!("[SISTEMA] Metrónomo iniciado: 60s/1200A/60P.");

    loop {
        ticker.tick().await;
        let ts = Local::now().format("%H:%M:%S%.3f").to_string();
        
        // --- 1. CAPTURA OBLIGATORIA (Tick = Aceleración) ---
        current_data.acceleration.push(AccelSample {
            timestamp: ts.clone(),
            measure: acc_module.get_latest_data(),
        });

        // --- 2. PRESIÓN (Cada segundo exacto: 20, 40, 60...) ---
        if (ticks + 1) % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                current_data.pressure.push(PressureSample {
                    timestamp: ts.clone(),
                    measure: Arc::new(s.buffers[s.latest_idx]),
                });
            }
            // Log de auditoría cada segundo
            print!("\r[{}] Tick: {:>4} | A:{:>4} | P:{:>2}", ts, ticks + 1, current_data.acceleration.len(), current_data.pressure.len());
            io::stdout().flush().ok();
        }

        // --- 3. AMBIENTE (Cada 20 segundos) ---
        if (ticks + 1) % 400 == 0 {
            let (t, h) = env_module.get_latest_avg();
            current_data.environment.push(EnvironmentSample {
                timestamp: ts.clone(),
                temperature: t,
                humidity: h,
            });
        }

        ticks += 1;

        // --- 4. CIERRE AL MINUTO (Tick 1200) ---
        if ticks >= 1200 {
            let finish_ts = ts.clone();
            
            // Verificación de integridad antes de enviar
            println!("\n[CIERRE] {} | A: {} | P: {}", finish_ts, current_data.acceleration.len(), current_data.pressure.len());

            let session = SessionSchema {
                initTimestamp: init_ts.clone(),
                finishTimestamp: finish_ts.clone(),
                dataRaw: std::mem::take(&mut current_data),
            };

            let _ = file_tx.try_send(session);

            // Reinicio de ciclo
            init_ts = finish_ts;
            ticks = 0;
            current_data.acceleration.reserve(1200);
            current_data.pressure.reserve(60);
        }
    }
}