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

        // Bus I2C directo para evitar fallos de detección de modelo en la Pi
        let i2c = I2c::with_bus(1).expect("Error crítico: No se pudo abrir el bus I2C 1");
        let shared_i2c = Arc::new(Mutex::new(i2c));

        // Inicialización de módulos
        self.audio.init(); 
        self.acceleration.init();
        self.environment.init(Arc::clone(&shared_i2c));
        self.pressure.init(Arc::clone(&shared_i2c));

        logger("CAPTURE", "Todos los periféricos vinculados correctamente.");
    }

    pub fn start(&self) {
        logger("CAPTURE", "Lanzando hilos de captura...");
        
        self.audio.start();
        self.environment.run();
        self.acceleration.run();
        self.pressure.run();
    }
}