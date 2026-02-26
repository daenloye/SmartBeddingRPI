mod pressure;
mod bluetooth;
mod config;
mod storage;

use storage::Storage;
use config::CONFIG;
use pressure::{PressureMatrix, COL_SIZE, ROW_SIZE};

use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use chrono::Local;

#[tokio::main]
async fn main() {
    // 1. Inicializar almacenamiento (Crea carpetas y prepara el buffer)
    let mut storage = Storage::init();

    // 2. Inicialización del sensor
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init().expect("[PRESSURE] Error crítico: No se pudo inicializar el hardware I2C")
    ));

    // Canal MPSC: Definimos el tipo explícitamente para evitar errores de inferencia
    let (pressure_tx, mut pressure_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(1);

    // --- HILO 1: HARDWARE (Lectura intensiva) ---
    let sensor_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        if CONFIG.debug_mode { println!("[DEBUG] Hilo de hardware iniciado."); }
        loop {
            if let Ok(mut s) = sensor_hw.write() {
                s.scan_and_update();
            }
            thread::sleep(Duration::from_millis(CONFIG.scan_delay_ms));
        }
    });

    // --- HILO 2: CONSUMIDOR (Renderizado y Storage en memoria) ---
    tokio::spawn(async move {
        while let Some((ts, matriz)) = pressure_rx.recv().await {
            // Renderizamos en consola
            renderizar_matriz(ts.clone(), matriz);
            
            // Almacenamos en el buffer de memoria si está habilitado
            if CONFIG.storage_enabled {
                storage.add_sample(ts, matriz);
            }
        }
    });

    // --- HILO 4: SERVICIO BLUETOOTH ---
    tokio::spawn(async {
        if let Err(e) = bluetooth::run_bluetooth_service().await {
            eprintln!("[ERROR] Bluetooth: {}", e);
        }
    });

    // --- HILO 3: EL METRÓNOMO (Controlador de flujo) ---
    let mut milis_intervalo = interval(Duration::from_millis(CONFIG.pressure_trigger_ms));
    milis_intervalo.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        milis_intervalo.tick().await;
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        
        if let Ok(s) = pressure_sensor.read() {
            let copia = s.buffers[s.latest_idx];
            // Enviamos la copia al hilo consumidor
            let _ = pressure_tx.try_send((timestamp, copia));
        }
    }
}

fn renderizar_matriz(ts: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
    let mut output = String::with_capacity(2048);
    output.push_str("\x1B[2J\x1B[H"); 
    output.push_str(&format!("─── MUESTRA: {} (Threshold: {}) ───\n\n", ts, CONFIG.pressure_threshold));

    for row in matrix.iter() {
        for &val in row.iter() {
            if val > CONFIG.pressure_threshold {
                output.push_str(&format!("\x1B[1;32m{:5}\x1B[0m ", val));
            } else {
                output.push_str(&format!("{:5} ", val));
            }
        }
        output.push_str("\n");
    }
    print!("{}", output);
}