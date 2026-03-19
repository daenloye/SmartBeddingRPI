mod capture; 
mod storage;
mod bridge;
mod utils;
mod interfaces;
mod mqtt;

use capture::CaptureController;
use storage::StorageController;
use bridge::BridgeController;
use mqtt::MqttController;
use utils::logger;

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    logger("SISTEMA", "=== INICIANDO SMART BEDDING SYSTEM (TOKIO) ===");

    // 1. Instanciamos los controladores (en el stack del main)
    let mut capture = CaptureController::new();
    let mut storage = StorageController::new();
    let mut bridge = BridgeController::new();
    let mqtt = MqttController::new();

    logger("SISTEMA", "Configurando hardware y carpetas...");

    // 2. Inicialización (Síncrona, prepara descriptores y buses)
    storage.init();
    capture.init(); 
    bridge.init();
    mqtt.init();

    // 3. Movemos a Arc para compartir entre hilos y tareas
    let shared_capture = Arc::new(capture);
    let shared_storage = Arc::new(storage);
    let shared_mqtt = Arc::new(mqtt);

    // 4. Arrancar motores
    // Esto lanza los hilos de std::thread (Hardware)
    shared_capture.start(); 
    
    // Esto lanza la tarea de tokio (Red)
    shared_mqtt.start();
    
    // --- ESTABILIZACIÓN ---
    logger("SISTEMA", "Esperando a que los sensores se estabilicen...");
    sleep(Duration::from_millis(2000)).await; 
    // ----------------------

    // 5. Arrancar el puente (Bridge)
    // Pasamos los Arcs. El bridge ahora tiene acceso a todo pero tú decides cuándo usarlo.
    bridge.start(
        Arc::clone(&shared_capture), 
        Arc::clone(&shared_storage),
    );

    logger("SISTEMA", "Sistema en ejecución y estable. Presione Ctrl+C para salir.");

    // 6. El "Ancla": Mantiene el proceso vivo eficientemente
    // Se queda aquí colgado hasta que detecta el Ctrl+C del sistema.
    tokio::signal::ctrl_c().await.expect("Fallo al escuchar señal CTRL+C");

    logger("SISTEMA", "Cerrando sistema (Salida abrupta detectada)...");
}