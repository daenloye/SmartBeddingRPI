use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rppal::i2c::I2c;
use crate::utils::logger;

pub struct EnvironmentModule {
    i2c: Option<Arc<Mutex<I2c>>>,
    // Almacena el promedio actual (Temp, Hum)
    pub current_data: Arc<Mutex<(f32, f32)>>,
}

impl EnvironmentModule {
    pub fn new() -> Self {
        Self {
            i2c: None,
            current_data: Arc::new(Mutex::new((0.0, 0.0))),
        }
    }

    pub fn init(&mut self, shared_i2c: Arc<Mutex<I2c>>) {
        logger("ENVIRONMENT", "Vinculando bus I2C para sensor ambiental...");
        self.i2c = Some(shared_i2c);
        logger("ENVIRONMENT", "Módulo configurado y listo.");
    }

    pub fn run(&self) {
        let i2c_mutex = self.i2c.as_ref().expect("I2C no inicializado").clone();
        let data_mutex = self.current_data.clone();
        let address = 0x44;

        logger("ENVIRONMENT", "Iniciando hilo de muestreo con promedio móvil...");

        thread::spawn(move || {
            // Ventana de 3 muestras para suavizar picos
            let mut samples: Vec<(f32, f32)> = Vec::with_capacity(3);

            loop {
                let start_time = Instant::now();

                match Self::read_sensor_hardware(&i2c_mutex, address) {
                    Ok((temp, hum)) => {
                        // 1. Gestión de la ventana de promedio
                        if samples.len() >= 3 { samples.remove(0); }
                        samples.push((temp, hum));

                        // 2. Calcular promedio
                        let count = samples.len() as f32;
                        let avg_t = samples.iter().map(|s| s.0).sum::<f32>() / count;
                        let avg_h = samples.iter().map(|s| s.1).sum::<f32>() / count;

                        // 3. Actualizar variable compartida
                        if let Ok(mut guard) = data_mutex.lock() {
                            *guard = (avg_t, avg_h);
                        }
                    }
                    Err(e) => logger("ENVIRONMENT", &format!("Error: {}", e)),
                }

                // El sensor ambiental no necesita mucha frecuencia (10s está bien)
                let elapsed = start_time.elapsed();
                let period = Duration::from_secs(10);
                if elapsed < period {
                    thread::sleep(period - elapsed);
                }
            }
        });
    }

    fn read_sensor_hardware(i2c_mutex: &Arc<Mutex<I2c>>, addr: u16) -> Result<(f32, f32), String> {
        // --- PASO 1: ENVIAR COMANDO ---
        {
            let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex I2C bloqueado")?;
            i2c.set_slave_address(addr).map_err(|e| e.to_string())?;
            // Comando 0x2400: High Repeatability
            i2c.write(&[0x24, 0x00]).map_err(|e| e.to_string())?;
        } // El mutex se libera aquí

        // --- PASO 2: ESPERA DE CONVERSIÓN ---
        // El sensor tarda un tiempo en procesar la lectura internamente
        thread::sleep(Duration::from_millis(50));

        // --- PASO 3: LEER DATOS ---
        let mut data = [0u8; 6];
        {
            let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex I2C bloqueado")?;
            
            /* CRÍTICO: Volvemos a poner la dirección 0x44. 
               Si no hacemos esto, y el hilo de PRESIÓN habló con el ADC (0x48) 
               mientras dormíamos los 50ms, el I2c intentará leer del ADC 
               y nos dará ceros o basura.
            */
            i2c.set_slave_address(addr).map_err(|e| e.to_string())?;
            i2c.read(&mut data).map_err(|e| e.to_string())?;
        }

        // Validación de seguridad
        if data.iter().all(|&x| x == 0) {
            return Err("Lectura nula (el bus devolvió ceros)".into());
        }

        // Conversión según Datasheet del SHT3x
        let raw_temp = u16::from_be_bytes([data[0], data[1]]);
        let raw_hum = u16::from_be_bytes([data[3], data[4]]);

        let temp = -45.0 + (175.0 * (raw_temp as f32 / 65535.0));
        let hum = 100.0 * (raw_hum as f32 / 65535.0);

        Ok((temp, hum))
    }

    pub fn get_latest(&self) -> (f32, f32) {
        *self.current_data.lock().unwrap_or_else(|e| e.into_inner())
    }
}