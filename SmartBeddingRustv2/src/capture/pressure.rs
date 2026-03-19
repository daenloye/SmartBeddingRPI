use crate::utils::logger;

pub struct PressureModule {}
impl PressureModule {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&self) {
        logger("PRESSURE", "Configurando dispositivo de entrada de audio...");
    }
}