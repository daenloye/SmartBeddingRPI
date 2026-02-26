use rppal::i2c::I2c;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::config::CONFIG;

pub struct EnvironmentModule {
    current_avg: Arc<Mutex<(f32, f32)>>,
}

impl EnvironmentModule {
    pub fn new(shared_i2c: Arc<Mutex<I2c>>) -> Self {
        let current_avg = Arc::new(Mutex::new((0.0, 0.0)));
        let current_avg_clone = Arc::clone(&current_avg);

        thread::spawn(move || {
            let address = 0x44;
            let mut samples: Vec<(f32, f32)> = Vec::with_capacity(3);

            loop {
                let start_cycle = Instant::now();
                if let Ok((temp, hum)) = Self::read_sensor(&shared_i2c, address) {
                    if samples.len() >= 3 { samples.remove(0); }
                    samples.push((temp, hum));

                    let count = samples.len() as f32;
                    let avg_t = samples.iter().map(|s| s.0).sum::<f32>() / count;
                    let avg_h = samples.iter().map(|s| s.1).sum::<f32>() / count;

                    if let Ok(mut guard) = current_avg_clone.lock() {
                        *guard = (avg_t, avg_h);
                    }
                }

                let elapsed = start_cycle.elapsed();
                let period = Duration::from_millis(CONFIG.environment_period_ms);
                if elapsed < period { thread::sleep(period - elapsed); }
            }
        });

        Self { current_avg }
    }

fn read_sensor(i2c_mutex: &Arc<Mutex<I2c>>, addr: u16) -> Result<(f32, f32), String> {
        // 1. ENVIAR COMANDO
        {
            let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex bloqueado")?;
            i2c.set_slave_address(addr).map_err(|e| e.to_string())?;
            // 0x2400: High Repeatability (sin clock stretching)
            i2c.write(&[0x24, 0x00]).map_err(|e| e.to_string())?;
            // El lock se libera al salir de este bloque
        }

        // 2. ESPERA DE CONVERSIÓN (El datasheet pide 15ms, daremos 50ms por seguridad)
        thread::sleep(Duration::from_millis(50));

        // 3. LEER RESULTADOS
        let mut data = [0u8; 6];
        {
            let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex bloqueado")?;
            // RE-ASEGURAR la dirección antes de leer, por si el hilo de presión la cambió
            i2c.set_slave_address(addr).map_err(|e| e.to_string())?;
            i2c.read(&mut data).map_err(|e| e.to_string())?;
        }

        // Si todos los bytes son 0, algo falló en el bus
        if data.iter().all(|&x| x == 0) {
            return Err("Lectura nula (all zeros)".to_string());
        }

        let raw_temp = u16::from_be_bytes([data[0], data[1]]);
        let raw_hum = u16::from_be_bytes([data[3], data[4]]);

        let temp = -45.0 + (175.0 * (raw_temp as f32 / 65535.0));
        let hum = 100.0 * (raw_hum as f32 / 65535.0);

        Ok((temp, hum))
    }

    pub fn get_latest_avg(&self) -> (f32, f32) {
        *self.current_avg.lock().unwrap_or_else(|e| e.into_inner())
    }
}