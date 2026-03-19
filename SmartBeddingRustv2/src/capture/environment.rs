use std::sync::{Arc, Mutex};
use rppal::i2c::I2c;
use crate::utils::logger;

pub struct EnvironmentModule {
    // Mutex para compartir el I2C con otros módulos
    i2c: Option<Arc<Mutex<I2c>>>,
}

impl EnvironmentModule {
    pub fn new() -> Self {
        Self {
            i2c: None,
        }
    }

    /// Prepara el periférico I2C para este módulo
    pub fn init(&mut self, shared_i2c: Arc<Mutex<I2c>>) {
        logger("ENVIRONMENT", "Vinculando bus I2C para sensor ambiental...");
        
        // Guardamos la referencia al bus compartido
        self.i2c = Some(shared_i2c);
        
        // Aquí podrías añadir una prueba rápida de conexión (opcional)
        logger("ENVIRONMENT", "Módulo configurado y listo para lectura.");
    }

    // El método 'run' lo dejaremos para después, como pediste.
}