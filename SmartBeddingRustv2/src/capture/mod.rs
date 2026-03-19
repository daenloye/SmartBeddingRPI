pub mod audio;
pub mod pressure;
pub mod acceleration;
pub mod environment;

use audio::AudioModule;
use pressure::PressureModule;
use acceleration::AccelerationModule;
use environment::EnvironmentModule;
use crate::utils::logger; 

use rppal::i2c::I2c;
use std::sync::{Arc, Mutex};

pub struct CaptureController {
    pub audio: AudioModule,
    pub pressure: PressureModule,
    pub acceleration: AccelerationModule,
    pub environment: EnvironmentModule,
}

impl CaptureController {
    pub fn new() -> Self {
        Self {
            audio: AudioModule::new(),
            pressure: PressureModule::new(),
            acceleration: AccelerationModule::new(),
            environment: EnvironmentModule::new(),
        }
    }

    pub fn init(&mut self) {
        logger("CAPTURE", "Iniciando hardware compartido...");

        // CAMBIO AQUÍ: No uses I2c::new(). Usa I2c::with_bus(1)
        // Esto ignora la detección del modelo y abre el dispositivo /dev/i2c-1 directamente.
        let i2c = I2c::with_bus(1).expect("Error crítico: No se pudo abrir el bus I2C 1");
        let shared_i2c = Arc::new(Mutex::new(i2c));

        // Inicializamos módulos
        self.audio.init(); 
        self.acceleration.init();
        
        // Pasamos el bus solo a quien lo necesita actualmente
        self.environment.init(Arc::clone(&shared_i2c));

        // Pressure sigue igual, sin el bus por ahora
        self.pressure.init(Arc::clone(&shared_i2c));

        logger("CAPTURE", "Todos los periféricos vinculados correctamente.");
    }

    pub fn start(&self) {
        logger("CAPTURE", "Lanzando hilos de sensores...");
        self.environment.run();
        self.acceleration.run();
        self.pressure.run();
    }


}