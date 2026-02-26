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
use tokio::signal;
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

    let (pressure_tx, mut pressure_rx) = mpsc::channel::<(String, Arc<[[u16; COL_SIZE]; ROW_SIZE]>)>(100);
    let (accel_tx, mut accel_rx) = mpsc::channel::<(String, [f32; 6])>(1300);
    let (env_tx, mut env_rx) = mpsc::channel::<(String, f32, f32)>(10);
    let (control_tx, mut control_rx) = mpsc::channel::<String>(10);

    // HILO HARDWARE: No lo tocamos, corre en background
    let sensor_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = sensor_hw.write() { s.scan_and_update(); }
            thread::sleep(Duration::from_millis(CONFIG.scan_delay_ms));
        }
    });

    // CONSUMIDOR DE AUDITORÍA Y STORAGE (Aquí se hace el trabajo sucio del log)
    tokio::spawn(async move {
        let mut p_count = 0;
        let mut a_count = 0;
        loop {
            tokio::select! {
                Some(cmd) = control_rx.recv() => {
                    if cmd == "FLUSH" { 
                        storage.flush_chunk(); 
                        println!("\n[{}] --- ARCHIVO GUARDADO (P: {}, A: {}) ---", Local::now().format("%H:%M:%S%.3f"), p_count, a_count);
                        p_count = 0; a_count = 0;
                    }
                    else if cmd == "SHUTDOWN" { storage.flush_chunk(); std::process::exit(0); }
                }
                Some((ts, m_ptr)) = pressure_rx.recv() => {
                    p_count += 1;
                    storage.add_pressure_sample(ts.clone(), m_ptr);
                    // Imprimimos la auditoría aquí para NO frenar el metrónomo
                    println!("[{}] AUDITORÍA -> P: {:>2} | A: {:>4}", ts, p_count, a_count);
                }
                Some((ts, d)) = accel_rx.recv() => {
                    a_count += 1;
                    storage.add_accel_sample(ts, d);
                }
                Some((ts, t, h)) = env_rx.recv() => {
                    storage.add_env_sample(ts, t, h);
                }
            }
        }
    });

    let mut ticker = interval(Duration::from_millis(50)); 
    // BURST: Si el sistema se traba, dispara los ticks acumulados rápido para recuperar
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);
    
    let mut ticks_count: u64 = 0;
    println!("[SISTEMA] Metrónomo iniciado. Prioridad de tiempo activa.");

    loop {
        ticker.tick().await;
        let ts = Local::now().format("%H:%M:%S%.3f").to_string();
        ticks_count += 1;

        // 1. Aceleración (20Hz) - Clonar Arc es barato, enviar por canal es rápido
        let _ = accel_tx.try_send((ts.clone(), acc_module.get_latest_data()));

        // 2. Ambiente (Cada 20s)
        if ticks_count % 400 == 0 {
            let e_m = Arc::clone(&env_module);
            let tx_e = env_tx.clone();
            let ts_e = ts.clone();
            tokio::spawn(async move {
                let (t, h) = e_m.get_latest_avg();
                let _ = tx_e.send((ts_e, t, h)).await;
            });
        }

        // 3. Presión (Cada 1s)
        if ticks_count % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                let ptr = Arc::new(s.buffers[s.latest_idx]);
                let _ = pressure_tx.try_send((ts.clone(), ptr));
            }

            if ticks_count >= 1200 {
                let _ = control_tx.send("FLUSH".to_string()).await;
                ticks_count = 0;
            }
        }
    }
}