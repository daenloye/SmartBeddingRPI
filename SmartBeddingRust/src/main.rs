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
use chrono::Local;

#[tokio::main]
async fn main() {
    let storage_dir = Storage::init_path();
    
    // Canal para comunicar el Metrónomo con el Worker de Procesamiento
    let (tx, mut rx) = mpsc::channel::<SessionSchema>(10);

    // --- WORKER NATIVO: IA + ESCRITURA ---
    let dir_clone = storage_dir.clone();
    thread::spawn(move || {
        let mut file_count = 1;
        println!("[WORKER] Hilo nativo de procesamiento iniciado.");
        
        // El blocking_recv() hace que este hilo no consuma CPU hasta que llegue data
        while let Some(mut session) = rx.blocking_recv() {
            let start_proc = Local::now();
            
            // --- AQUÍ METES TU IA / PROCESAMIENTO ---
            // Como dijiste, aquí puedes demorarte 20 segundos y no pasa nada.
            procesar_inteligencia_artificial(&mut session);
            
            let proc_dur = Local::now().signed_duration_since(start_proc).num_milliseconds();

            // --- ESCRITURA FINAL ---
            let path = dir_clone.join(format!("reg_{}.json", file_count));
            if let Ok(file) = File::create(&path) {
                if serde_json::to_writer(file, &session).is_ok() {
                    println!(
                        "\n[WORKER] Bloque {} guardado. IA: {}ms | Path: {}", 
                        file_count, proc_dur, path.display()
                    );
                }
            }
            file_count += 1;
        }
    });

    // --- SETUP HARDWARE ---
    let shared_i2c = Arc::new(Mutex::new(I2c::new().expect("I2C Fail")));
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    let env_module = Arc::new(EnvironmentModule::new(Arc::clone(&shared_i2c)));
    let pressure_sensor: Arc<RwLock<PressureMatrix>> = Arc::new(RwLock::new(
        PressureMatrix::init(Arc::clone(&shared_i2c)).expect("Pressure Fail")
    ));

    // Hilo de escaneo constante (Presión)
    let p_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = p_hw.write() { s.scan_and_update(); }
            thread::sleep(Duration::from_millis(10));
        }
    });

    // --- METRÓNOMO ---
    let mut ticker = interval(Duration::from_millis(50));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);

    let mut current_data = DataRaw::default();
    let mut init_ts = Local::now().format("%H:%M:%S%.3f").to_string();
    let mut ticks = 0;

    println!("[SISTEMA] Metrónomo en marcha. 1200A/60P por minuto.");

    loop {
        ticker.tick().await;
        let ts = Local::now().format("%H:%M:%S%.3f").to_string();
        
        // 1. Aceleración
        current_data.acceleration.push(AccelSample {
            timestamp: ts.clone(),
            measure: acc_module.get_latest_data(),
        });

        // 2. Presión (cada 1s)
        if (ticks + 1) % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                current_data.pressure.push(PressureSample {
                    timestamp: ts.clone(),
                    measure: Arc::new(s.buffers[s.latest_idx]),
                });
            }
            print!("\r[{}] Ticks: {:>4} | A:{} P:{}", ts, ticks + 1, current_data.acceleration.len(), current_data.pressure.len());
            io::stdout().flush().ok();
        }

        // 3. Ambiente (cada 20s)
        if (ticks + 1) % 400 == 0 {
            let (t, h) = env_module.get_latest_avg();
            current_data.environment.push(EnvironmentSample {
                timestamp: ts.clone(),
                temperature: t,
                humidity: h,
            });
        }

        ticks += 1;

        // 4. Envío al Worker
        if ticks >= 1200 {
            let finish_ts = ts.clone();
            let session = SessionSchema {
                initTimestamp: init_ts.clone(),
                finishTimestamp: finish_ts.clone(),
                dataRaw: std::mem::take(&mut current_data),
            };

            // Se envía la data y el metrónomo queda libre para el siguiente minuto
            if let Err(_) = tx.try_send(session) {
                println!("\n[ALERTA] Canal saturado. El Worker es demasiado lento.");
            }

            init_ts = finish_ts;
            ticks = 0;
            current_data.acceleration.reserve(1200);
            current_data.pressure.reserve(60);
        }
    }
}

// Aquí es donde meterás tu magia de procesamiento
fn procesar_inteligencia_artificial(session: &mut SessionSchema) {
    // Ejemplo de acceso: session.dataRaw.acceleration
    // Este código corre en el hilo nativo.
    // thread::sleep(Duration::from_secs(5)); // Descomenta para probar el lag
}