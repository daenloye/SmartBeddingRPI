use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rppal::i2c::I2c;
use crate::utils::logger;

pub struct EnvironmentModule {
    i2c: Option<Arc<Mutex<I2c>>>,
    // Variable local para almacenar la última lectura (Temp, Hum)
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

    /// Lógica de lectura y muestreo cada 10 segundos
    pub fn run(&self) {
        // Clonamos las referencias para el hilo
        let i2c_mutex = self.i2c.as_ref().expect("I2C no inicializado").clone();
        let data_mutex = self.current_data.clone();
        let address = 0x44; // Dirección del SHT3x

        logger("ENVIRONMENT", "Iniciando hilo de muestreo (10s)...");

        thread::spawn(move || {
            loop {
                let start_time = Instant::now();

                // 1. Ejecutar lectura física del sensor
                match Self::read_sensor_hardware(&i2c_mutex, address) {
                    Ok((temp, hum)) => {
                        if let Ok(mut guard) = data_mutex.lock() {
                            *guard = (temp, hum);
                            //logger("ENVIRONMENT", &format!("Lectura OK: {:.2}°C, {:.2}% RH", temp, hum));
                        }
                    }
                    Err(e) => logger("ENVIRONMENT", &format!("Error de lectura: {}", e)),
                }

                // 2. Control del periodo de 10 segundos
                let elapsed = start_time.elapsed();
                let period = Duration::from_secs(10);
                if elapsed < period {
                    thread::sleep(period - elapsed);
                }
            }
        });
    }

    /// Lógica de bajo nivel para el bus I2C (basada en tu sistema anterior)
    fn read_sensor_hardware(i2c_mutex: &Arc<Mutex<I2c>>, addr: u16) -> Result<(f32, f32), String> {
        let mut data = [0u8; 6];

        {
            let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex I2C bloqueado")?;
            i2c.set_slave_address(addr).map_err(|e| e.to_string())?;
            // Comando de lectura: High Repeatability
            i2c.write(&[0x24, 0x00]).map_err(|e| e.to_string())?;
        }

        // Tiempo de conversión del sensor
        thread::sleep(Duration::from_millis(50));

        {
            let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex I2C bloqueado")?;
            i2c.read(&mut data).map_err(|e| e.to_string())?;
        }

        if data.iter().all(|&x| x == 0) {
            return Err("Lectura nula".into());
        }

        let raw_temp = u16::from_be_bytes([data[0], data[1]]);
        let raw_hum = u16::from_be_bytes([data[3], data[4]]);

        let temp = -45.0 + (175.0 * (raw_temp as f32 / 65535.0));
        let hum = 100.0 * (raw_hum as f32 / 65535.0);

        Ok((temp, hum))
    }

    /// Método para que otros módulos consulten el último valor sin bloquear el hilo
    pub fn get_latest(&self) -> (f32, f32) {
        return *self.current_data.lock().unwrap_or_else(|e| e.into_inner());
    }
}