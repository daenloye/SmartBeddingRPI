//cross build --target aarch64-unknown-linux-gnu --release
use chrono::Local;

mod capture; 
mod storage;
mod bridge;
mod utils;

use capture::CaptureController;
use storage::StorageController;
use bridge::BridgeController;
use utils::logger; // Importamos el logger para usarlo aquí también

use std::thread;
use std::time::Duration;
use std::sync::Arc;

fn main() {
    logger("SISTEMA", "INICIANDO SISTEMA");

    // 1. Creamos los controladores dentro de un Arc para compartirlos entre hilos
    let mut capture = CaptureController::new();
    let mut storage = StorageController::new();
    let mut bridge = BridgeController::new();

    logger("SISTEMA", "INICIANDO MÓDULOS");

    // Inicialización normal (antes de moverlos a los Arcs)
    storage.init();
    capture.init();
    bridge.init(); // Si necesitas algo previo

    // 2. Envolvemos en Arc para el Bridge
    let shared_capture = Arc::new(capture);
    let shared_storage = Arc::new(storage);

    // 3. Arrancar motores
    shared_capture.start(); // El Sampler (Muestreador) empieza en Capture
    bridge.start(Arc::clone(&shared_capture), Arc::clone(&shared_storage));

    logger("SISTEMA", "Estado de recolección: Activo");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}