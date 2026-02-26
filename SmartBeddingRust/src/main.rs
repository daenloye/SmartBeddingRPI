mod pressure;
mod config;
mod storage;
mod acceleration;

use storage::Storage;
use config::CONFIG;
use pressure::{PressureMatrix, COL_SIZE, ROW_SIZE};
use acceleration::AccelerationModule;
use rppal::spi::{Bus, SlaveSelect};

use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use chrono::Local;

#[tokio::main]
async fn main() {
    let mut storage = Storage::init();
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init().expect("[PRESSURE] Error crítico")
    ));

    let (pressure_tx, mut pressure_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(10);
    let (accel_tx, mut accel_rx) = mpsc::channel::<(String, [f32; 6])>(100);
    
    // Canal de visualización
    let (visual_tx, mut visual_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(1);

    // --- HILO 1: HARDWARE PRESIÓN ---
    let sensor_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = sensor_hw.write() { s.scan_and_update(); }
            thread::sleep(Duration::from_millis(CONFIG.scan_delay_ms));
        }
    });

    // --- HILO 2: STORAGE ---
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

    // --- HILO: RENDERIZADOR (Solo si está habilitado) ---
    if CONFIG.pressure_matrix_visualization {
        tokio::spawn(async move {
            println!("[SISTEMA] Visualización de matriz ACTIVADA.");
            while let Some((ts, matriz)) = visual_rx.recv().await {
                renderizar_matriz(ts, matriz);
            }
        });
    } else {
        println!("[SISTEMA] Visualización de matriz DESACTIVADA (Modo ahorro/log).");
    }

    // --- HILO 3: METRÓNOMO MAESTRO ---
    let mut ticker = interval(Duration::from_millis(CONFIG.acceleration_trigger_ms)); 
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut ticks_count: u64 = 0;

    loop {
        ticker.tick().await;
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        
        // A. ACELERACIÓN
        let a_data = acc_module.get_latest_data();
        let _ = accel_tx.send((timestamp.clone(), a_data)).await;

        // B. PRESIÓN
        ticks_count += 1;
        if ticks_count % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                let p_data = s.buffers[s.latest_idx];
                
                // Enviar a Storage
                let _ = pressure_tx.send((timestamp.clone(), p_data)).await;
                
                // Enviar a Visualización (solo si el config lo permite)
                if CONFIG.pressure_matrix_visualization {
                    let _ = visual_tx.try_send((timestamp.clone(), p_data));
                }
            }
        }
    }
}

fn renderizar_matriz(ts: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
    let mut output = String::with_capacity(2048);
    // Limpieza de terminal usando secuencias ANSI
    output.push_str("\x1B[2J\x1B[H"); 
    output.push_str(&format!("─── MATRIZ EN TIEMPO REAL: {} ───\n\n", ts));

    for row in matrix.iter() {
        for &val in row.iter() {
            if val > CONFIG.pressure_threshold {
                output.push_str(&format!("\x1B[1;32m{:4}\x1B[0m ", val)); // Verde si activo
            } else {
                output.push_str(&format!("{:4} ", val));
            }
        }
        output.push_str("\n");
    }
    print!("{}", output);
}