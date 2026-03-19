mod capture; 
mod storage;
mod bridge;
mod utils;
mod interfaces;

use capture::CaptureController;
use storage::StorageController;
use bridge::BridgeController;
use utils::logger;

use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    logger("SISTEMA", "=== INICIANDO SMART BEDDING SYSTEM ===");

    // 1. Variable de control compartida
    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);

    // Opcional: Manejador de Ctrl+C para cerrar limpiamente
    // Si tienes la crate 'ctrlc' en Cargo.toml, descomenta esto:
    /*
    ctrlc::set_handler(move || {
        logger("SISTEMA", "Señal de parada recibida (Ctrl+C)...");
        r.store(false, Ordering::SeqCst);
    }).expect("Error configurando manejador de señales");
    */

    // 2. Instanciamos los controladores
    let mut capture = CaptureController::new();
    let mut storage = StorageController::new();
    let mut bridge = BridgeController::new();

    logger("SISTEMA", "Configurando hardware y carpetas...");

    // 3. Inicialización
    storage.init();
    capture.init();
    bridge.init();

    // 4. Movemos a Arc
    let shared_capture = Arc::new(capture);
    let shared_storage = Arc::new(storage);

    // 5. Arrancar motores
    // El Sampler interno de Capture empieza a llenar los Mutex
    shared_capture.start(); 
    
    // El Bridge empieza a "cosechar" los datos y mandarlos al Storage
    bridge.start(Arc::clone(&shared_capture), Arc::clone(&shared_storage));

    logger("SISTEMA", "Sistema en ejecución. Presione Ctrl+C para salir.");

    // 6. Loop principal controlado
    while running.load(Ordering::SeqCst) {
        // Aquí podrías añadir lógica de chequeo de salud del sistema
        thread::sleep(Duration::from_secs(1));
    }

    logger("SISTEMA", "Cerrando sistema de forma segura...");
    // Aquí podrías llamar a funciones de limpieza si fuera necesario
}