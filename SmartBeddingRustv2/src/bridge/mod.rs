use crate::capture::CaptureController;
use crate::storage::StorageController;
use crate::utils::logger;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct BridgeController {
    // Aquí guardaremos las referencias una vez se inicie el flujo
}

impl BridgeController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&mut self) {
        logger("BRIDGE", "Controlador de puente listo.");
    }

    /// Este es el método que orquestará el paso de datos
    pub fn start(&self, capture: Arc<CaptureController>, storage: Arc<StorageController>) {
        logger("BRIDGE", "Iniciando orquestación de datos...");

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            let mut tick_counter = 0;
            let tick_rate = Duration::from_millis(10); // Reloj maestro cada 10s

            loop {
                //Muestreo de giroscopio a 50ms
                if(tick_counter%50==0){

                }

                //Muestreo de environment a 10.000 ms
                if(tick_counter%10000==0){
                    logger("BRIDGE", "Muestreo de ambiente");
                    let (temp, hum) = capture.environment.get_latest();
                }

                //Muestreo de presion a 1000ms
                if(tick_counter%1000==0){

                }

                //Si ya llegó a 1 minuto cierro el archivo
                if(tick_counter%60000==0){
                    //Lo reinicio
                    tick_counter=0;
                }else{
                    //Lo aumento 10ms
                    tick_counter+=10;
                }
                



                // Control del metrónomo del puente
                let elapsed = last_tick.elapsed();
                if elapsed < tick_rate {
                    thread::sleep(tick_rate - elapsed);
                }
                last_tick = Instant::now();
            }
        });
    }
}