//cross build --target aarch64-unknown-linux-gnu --release
use chrono::Local;

mod capture; 
mod storage;
mod utils;

use capture::CaptureController;
use storage::StorageController;
use utils::logger; // Importamos el logger para usarlo aquí también

fn main() {
    logger("SISTEMA", "INICIANDO SISTEMA");

    // ----------------------------------------
    // Definición de variables
    // ----------------------------------------

    let mut collecting: bool = false;
    let mut controller = CaptureController::new();
    let mut storage = StorageController::new();

    // ----------------------------------------
    // Inicialización de módulos
    // ----------------------------------------

    logger("SISTEMA", "INICIANDO MÓDULOS");

    storage.init();
    controller.init();


    // ----------------------------------------
    // Definición de hilos
    // ----------------------------------------
    
    collecting = true;
    logger("SISTEMA", &format!("Estado de recolección: {}", collecting));
}