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

    let running = Arc::new(AtomicBool::new(true));
    
    // 2. Instanciamos los controladores
    let mut capture = CaptureController::new();
    let mut storage = StorageController::new();
    let mut bridge = BridgeController::new();

    logger("SISTEMA", "Configurando hardware y carpetas...");

    // 3. Inicialización de Hardware y Archivos
    storage.init();
    capture.init(); // Aquí se abren los buses I2C y GPIO
    bridge.init();

    // 4. Movemos a Arc para compartir entre hilos
    let shared_capture = Arc::new(capture);
    let shared_storage = Arc::new(storage);

    // 5. Arrancar motores de sensores
    // Esto lanza el hilo de la matriz de presión (que tarda ~1.5s en su primer scan)
    shared_capture.start(); 
    
    // --- ESTABILIZACIÓN ---
    logger("SISTEMA", "Esperando a que los sensores se estabilicen...");
    thread::sleep(Duration::from_millis(2000)); // 2 segundos de cortesía
    // ----------------------

    // 6. Arrancar el puente (Bridge)
    // Ahora, cuando el Bridge pida la primera muestra, el buffer ya tendrá datos reales
    bridge.start(Arc::clone(&shared_capture), Arc::clone(&shared_storage));

    logger("SISTEMA", "Sistema en ejecución y estable. Presione Ctrl+C para salir.");

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));
    }

    logger("SISTEMA", "Cerrando sistema de forma segura...");
}