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

fn main() {
    logger("SISTEMA", "INICIANDO SISTEMA");

    // ----------------------------------------
    // Definición de variables
    // ----------------------------------------

    let mut collecting: bool = false;
    let mut capture = CaptureController::new();
    let mut storage = StorageController::new();
    let mut bridge = BridgeController::new();

    // ----------------------------------------
    // Inicialización de módulos
    // ----------------------------------------

    logger("SISTEMA", "INICIANDO MÓDULOS");

    storage.init();
    bridge.init();
    capture.init();


    // ----------------------------------------
    // Definición de hilos
    // ----------------------------------------
    
    collecting = true;
    logger("SISTEMA", &format!("Estado de recolección: {}", collecting));

    capture.start();

    // Mantenemos el main vivo (por ahora con un loop simple)
    loop {
        thread::sleep(Duration::from_secs(60));
    }

}