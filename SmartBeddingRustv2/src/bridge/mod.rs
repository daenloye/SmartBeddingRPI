
use crate::utils::logger;

pub struct BridgeController {
}

impl BridgeController {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn init(&mut self) {
        logger("BRIDGE", "Inicializando controladores de puente...");
    }

}