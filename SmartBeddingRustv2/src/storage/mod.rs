// Importamos el logger desde la raíz del proyecto
use crate::utils::logger; 

pub struct StorageController {
}

impl StorageController {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn init(&mut self) {
        // Ahora podemos usar el logger aquí
        logger("STORAGE", "Inicializando controladores de almacenamiento...");
    }
}