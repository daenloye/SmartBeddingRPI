// cross build --target aarch64-unknown-linux-gnu --release
mod capture; 
mod storage;
mod bridge;
mod utils;

use capture::CaptureController;
use storage::StorageController;
use bridge::BridgeController;
use utils::logger;

use std::thread;
use std::time::Duration;
use std::sync::Arc;

fn main() {
    logger("SISTEMA", "INICIANDO SISTEMA");

    // 1. Instanciamos los controladores
    let mut capture = CaptureController::new();
    let mut storage = StorageController::new();
    let mut bridge = BridgeController::new();

    logger("SISTEMA", "INICIANDO MÓDULOS");

    // 2. Inicialización (Hardware, carpetas, etc.)
    storage.init();
    capture.init();
    bridge.init();

    // 3. Movemos a Arc para poder compartirlos entre hilos de forma segura
    let shared_capture = Arc::new(capture);
    let shared_storage = Arc::new(storage);

    // 4. Arrancar motores
    // El Sampler interno de Capture empieza a llenar los Mutex (ej. Accel a 25ms)
    shared_capture.start(); 
    
    // El Bridge empieza a "cosechar" los datos y mandarlos al Storage
    bridge.start(Arc::clone(&shared_capture), Arc::clone(&shared_storage));

    logger("SISTEMA", "Estado de recolección: Activo");

    // Mantenemos el hilo principal vivo
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}