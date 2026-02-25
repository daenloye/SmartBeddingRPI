mod pressure;
use pressure::{PressureMatrix, COL_SIZE, ROW_SIZE};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};
use tokio::sync::mpsc;
use chrono::Local;

#[tokio::main]
async fn main() {
    let sensor = Arc::new(RwLock::new(
        PressureMatrix::init().expect("Error I2C")
    ));

    // Canal para enviar la matriz al hilo de dibujo (capacidad 1 para que no se acumule basura)
    let (tx, mut rx) = mpsc::channel::<(String, [[u16; COL_SIZE]; ROW_SIZE])>(1);

    // --- HILO 1: HARDWARE (Nativo) ---
    let sensor_hw = Arc::clone(&sensor);
    thread::spawn(move || {
        loop {
            if let Ok(mut s) = sensor_hw.write() {
                s.scan_and_update();
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    // --- HILO 2: RENDERIZADO (Consola) ---
    tokio::spawn(async move {
        while let Some((ts, matriz)) = rx.recv().await {
            renderizar_matriz(ts, matriz);
        }
    });

    // --- HILO 3: EL METRÓNOMO (Main) ---
    let mut milis_intervalo = interval(Duration::from_secs(1));
    milis_intervalo.set_missed_tick_behavior(MissedTickBehavior::Skip);

    println!("Sistema operativo. Iniciando visualización segura...");

    loop {
        milis_intervalo.tick().await;
        
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        
        if let Ok(s) = sensor.read() {
            let copia = s.buffers[s.latest_idx];
            // Enviamos al dibujante. try_send no bloquea si el dibujante está ocupado.
            let _ = tx.try_send((timestamp, copia));
        }
    }
}

fn renderizar_matriz(ts: String, matrix: [[u16; COL_SIZE]; ROW_SIZE]) {
    let mut output = String::with_capacity(2048);
    output.push_str("\x1B[2J\x1B[H"); // Limpiar pantalla
    output.push_str(&format!("─── MUESTRA: {} ───\n\n", ts));

    for row in matrix.iter() {
        for &val in row.iter() {
            if val > 100 {
                output.push_str(&format!("\x1B[1;32m{:5}\x1B[0m ", val));
            } else {
                output.push_str(&format!("{:5} ", val));
            }
        }
        output.push_str("\n");
    }
    print!("{}", output);
}