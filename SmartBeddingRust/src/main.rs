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
    // 1. Inicialización de Almacenamiento y Módulos
    let mut storage = Storage::init();
    
    // Módulo de aceleración (Inicia su propio hilo nativo a 40Hz internamente)
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));
    
    // Módulo de presión (Hardware I2C)
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init().expect("[PRESSURE] Error crítico: No se pudo inicializar hardware I2C")
    ));

    // Canales de comunicación hacia el Storage
    let (pressure_tx, mut pressure_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(10);
    let (accel_tx, mut accel_rx) = mpsc::channel::<(String, [f32; 6])>(100);

    // --- HILO 1: ESCANEO HARDWARE PRESIÓN (Hilo Nativo) ---
    // Este hilo corre a la máxima velocidad permitida por el bus I2C para actualizar datos
    let sensor_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = sensor_hw.write() { 
                s.scan_and_update(); 
            }
            thread::sleep(Duration::from_millis(CONFIG.scan_delay_ms));
        }
    });

    // --- HILO 2: CONSUMIDOR ÚNICO DE STORAGE (Tokio Task) ---
    // Centraliza las escrituras en los vectores de memoria
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some((ts, matriz)) = pressure_rx.recv() => {
                    if CONFIG.storage_enabled {
                        storage.add_pressure_sample(ts, matriz);
                    }
                }
                Some((ts, data)) = accel_rx.recv() => {
                    if CONFIG.storage_enabled {
                        storage.add_accel_sample(ts, data);
                    }
                }
            }
        }
    });

    // --- HILO 3: METRÓNOMO MAESTRO (Sincronización Total) ---
    // Orquestador: decide cuándo se toma la foto de cada sensor
    let mut ticker = interval(Duration::from_millis(CONFIG.acceleration_trigger_ms)); // 50ms = 20Hz
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    
    let mut ticks_count: u64 = 0;

    println!("[SISTEMA] Iniciando captura sincronizada...");

    loop {
        ticker.tick().await;
        
        // Generamos un ÚNICO timestamp para este instante de tiempo
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        
        // A. CAPTURA ACELERACIÓN (Siempre a 20Hz)
        let a_data = acc_module.get_latest_data();
        let _ = accel_tx.send((timestamp.clone(), a_data)).await;

        // B. CAPTURA PRESIÓN (Cada 1 segundo / 20 ticks)
        ticks_count += 1;
        if ticks_count % 20 == 0 {
            if let Ok(s) = pressure_sensor.read() {
                let p_data = s.buffers[s.latest_idx];
                // Enviamos con el MISMO timestamp que la muestra de aceleración
                let _ = pressure_tx.send((timestamp.clone(), p_data)).await;
            }
            
            if CONFIG.debug_mode {
                // Log de latido del sistema
                println!("[MASTER] [{}] Sync OK - Accel Ticks: {}", timestamp, ticks_count);
            }
        }
    }
}

// --- FUNCIÓN DE RENDERIZADO (Para debugear visualmente la matriz) ---
fn _renderizar_matriz(ts: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
    let mut output = String::with_capacity(2048);
    output.push_str("\x1B[2J\x1B[H"); // Limpiar consola
    output.push_str(&format!("─── MUESTRA: {} (Threshold: {}) ───\n\n", ts, CONFIG.pressure_threshold));

    for row in matrix.iter() {
        for &val in row.iter() {
            if val > CONFIG.pressure_threshold {
                output.push_str(&format!("\x1B[1;32m{:5}\x1B[0m ", val)); // Verde si pasa el umbral
            } else {
                output.push_str(&format!("{:5} ", val));
            }
        }
        output.push_str("\n");
    }
    print!("{}", output);
}