mod pressure;
mod config;
mod storage;
mod acceleration;
mod environment;

use storage::Storage;
use config::CONFIG;
use pressure::{PressureMatrix, COL_SIZE, ROW_SIZE};
use acceleration::AccelerationModule;
use environment::EnvironmentModule;
use rppal::spi::{Bus, SlaveSelect};
use rppal::i2c::I2c;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use chrono::Local;

#[tokio::main]
async fn main() {
    let mut storage = Storage::init();
    let shared_i2c = Arc::new(Mutex::new(I2c::new().expect("I2C Fail")));

    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    let env_module = Arc::new(EnvironmentModule::new(Arc::clone(&shared_i2c)));
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init(Arc::clone(&shared_i2c)).expect("Pressure Fail")
    ));

    let (pressure_tx, mut pressure_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(20);
    let (accel_tx, mut accel_rx) = mpsc::channel::<(String, [f32; 6])>(100);
    let (visual_tx, mut visual_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(1);

    // --- HILO 1: HARDWARE PRESIÓN (Independiente) ---
    let sensor_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = sensor_hw.write() { s.scan_and_update(); }
            thread::sleep(Duration::from_millis(CONFIG.scan_delay_ms));
        }
    });

    // --- HILO 2: CONSUMIDOR STORAGE ---
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some((ts, matriz)) = pressure_rx.recv() => {
                    if CONFIG.storage_enabled { storage.add_pressure_sample(ts, matriz); }
                }
                Some((ts, data)) = accel_rx.recv() => {
                    if CONFIG.storage_enabled { storage.add_accel_sample(ts, data); }
                }
            }
        }
    });

    // --- HILO: RENDERIZADOR ---
    if CONFIG.pressure_matrix_visualization {
        tokio::spawn(async move {
            while let Some((ts, matriz)) = visual_rx.recv().await {
                renderizar_matriz(ts, matriz);
            }
        });
    }

    // --- HILO 3: METRÓNOMO MAESTRO (Reloj atómico) ---
    let mut ticker = interval(Duration::from_millis(CONFIG.acceleration_trigger_ms));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut ticks_count: u64 = 0;

    println!("[SISTEMA] Iniciando con parches de seguridad para RwLock...");

    loop {
        ticker.tick().await;
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();

        // 1. Aceleración
        let acc = Arc::clone(&acc_module);
        let tx_a = accel_tx.clone();
        let ts_a = timestamp.clone();
        tokio::spawn(async move {
            let data = acc.get_latest_data();
            let _ = tx_a.send((ts_a, data)).await;
        });

        ticks_count += 1;

        // 2. Presión (Cada 1s / 20 ticks)
        if ticks_count % 20 == 0 {
            let p_sensor = Arc::clone(&pressure_sensor);
            let tx_p = pressure_tx.clone();
            let tx_v = visual_tx.clone();
            let ts_p = timestamp.clone();
            let viz_enabled = CONFIG.pressure_matrix_visualization;

            tokio::spawn(async move {
                // EXPLICACIÓN: Obtenemos el dato y soltamos el candado INMEDIATAMENTE
                let data_opt = if let Ok(s) = p_sensor.read() {
                    Some(s.buffers[s.latest_idx])
                } else {
                    None
                };

                // Ahora que el candado está suelto, podemos hacer .await con seguridad
                if let Some(data) = data_opt {
                    let _ = tx_p.send((ts_p.clone(), data)).await;
                    if viz_enabled {
                        let _ = tx_v.try_send((ts_p, data));
                    }
                }
            });
        }

        // 3. Ambiente (Cada 20s / 400 ticks)
        if ticks_count % 400 == 0 {
            let env = Arc::clone(&env_module);
            let ts_e = timestamp.clone();
            tokio::spawn(async move {
                let (t, h) = env.get_latest_avg();
                println!("[MASTER-ENV] [{}] T: {:.2}°C | H: {:.2}%", ts_e, t, h);
            });
        }
    }
}

fn renderizar_matriz(ts: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
    let mut output = String::with_capacity(2048);
    output.push_str("\x1B[2J\x1B[H"); 
    output.push_str(&format!("─── MATRIZ: {} ───\n\n", ts));
    for row in matrix.iter() {
        for &val in row.iter() {
            if val > CONFIG.pressure_threshold {
                output.push_str(&format!("\x1B[1;32m{:4}\x1B[0m ", val));
            } else {
                output.push_str(&format!("{:4} ", val));
            }
        }
        output.push_str("\n");
    }
    print!("{}", output);
}