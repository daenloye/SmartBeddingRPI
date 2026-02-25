//cross build --target aarch64-unknown-linux-gnu --release

mod pressure; // Declaramos el módulo

use pressure::PressureMatrix;
use tokio::sync::mpsc;
use std::time::{Instant, Duration};

#[tokio::main]
async fn main() {
    println!("Iniciando lector de presión...");

    // Creamos un canal para pasar la matriz del hilo de hardware al hilo principal
    // mpsc = Multi-Producer, Single-Consumer
    let (tx, mut rx) = mpsc::channel(5);

    // Hilo de Hardware (Worker)
    tokio::task::spawn_blocking(move || {
        let mut sensor = PressureMatrix::init().expect("Fallo al inicializar hardware");
        
        loop {
            let data = sensor.scan();
            if tx.blocking_send(data).is_err() {
                break; // Si el receptor muere, paramos el loop
            }
        }
    });

    // Hilo de Procesamiento/UI (Main)
    let mut frame_count = 0;
    while let Some(matrix) = rx.recv().await {
        frame_count += 1;
        
        // Limpiar pantalla (ANSI escape)
        print!("\x1B[2J\x1B[H");
        println!("Frame: {} | Datos recibidos de la matriz", frame_count);

        for row in matrix.iter() {
            for &val in row.iter() {
                if val > 100 {
                    print!("\x1B[1;32m{:5}\x1B[0m ", val);
                } else {
                    print!("{:5} ", val);
                }
            }
            println!();
        }
        
        // Opcional: Pequeña pausa para no saturar la terminal
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}