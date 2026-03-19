use crate::utils::logger;

pub struct AudioModule {}

impl AudioModule {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&self) {
        logger("AUDIO", "Configurando dispositivo de entrada de audio...");
    }
}