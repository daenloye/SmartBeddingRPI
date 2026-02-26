mod pressure;
mod bluetooth;
mod config;
mod storage;
mod acceleration; // <-- Añadido

use storage::Storage;
use config::CONFIG;
use pressure::{PressureMatrix, COL_SIZE, ROW_SIZE};
use acceleration::AccelerationModule; // <-- Añadido
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

    // 1. Inicialización de Acelerómetro (SPI0, SS0)
    let acc_module = Arc::new(AccelerationModule::new(Bus::Spi0, SlaveSelect::Ss0));

    // 2. Inicialización del sensor de presión
    let pressure_sensor = Arc::new(RwLock::new(
        PressureMatrix::init().expect("[PRESSURE] Error crítico: No se pudo inicializar el hardware I2C")
    ));

    let (pressure_tx, mut pressure_rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(1);

    // --- HILO 1: HARDWARE PRESIÓN (Escaneo I2C constante) ---
    let sensor_hw = Arc::clone(&pressure_sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = sensor_hw.write() {
                s.scan_and_update();
            }
            thread::sleep(Duration::from_millis(CONFIG.scan_delay_ms));
        }
    });

    // --- HILO 2: CONSUMIDOR ACELERACIÓN (20Hz según trigger) ---
    let acc_consumer = Arc::clone(&acc_module);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(CONFIG.acceleration_trigger_ms));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        
        let mut count = 0;

        loop {
            ticker.tick().await;
            let _data = acc_consumer.get_latest_data(); 
            count += 1;

            // Imprimimos cada segundo (cada 20 muestras a 20Hz)
            if count % 20 == 0 {
                let ts = Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[ACCEL]   [{}] Muestras totales: {}", ts, count);
            }
        }
    });

    // --- HILO 3: CONSUMIDOR PRESIÓN ---
    tokio::spawn(async move {
        while let Some((ts, matriz)) = pressure_rx.recv().await {
            // Imprimimos el log de presión cada vez que llega una muestra (1Hz)
            println!("[PRESSURE][{}] Muestra de matriz recibida", ts);
            
            // renderizar_matriz(ts.clone(), matriz); // Comentado por ahora
            
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

    // --- HILO 5: EL METRÓNOMO PRESIÓN (1Hz según trigger) ---
    let mut milis_intervalo = interval(Duration::from_millis(CONFIG.pressure_trigger_ms));
    milis_intervalo.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        milis_intervalo.tick().await;
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        
        if let Ok(s) = pressure_sensor.read() {
            let copia = s.buffers[s.latest_idx];
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