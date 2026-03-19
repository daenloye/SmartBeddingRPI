use crate::utils::logger;

pub struct AccelerationModule {}
impl AccelerationModule {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&self) {
        logger("ACCELERATION", "Configurando dispositivo de entrada de audio...");
    }
}