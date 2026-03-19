use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::utils::logger;

// Registros del sensor (ICM-42605 o similar)
const REG_PWR_MGMT0: u8 = 0x4E;
const REG_GYRO_CONFIG0: u8 = 0x4F;
const REG_ACCEL_CONFIG0: u8 = 0x50;
const REG_ACCEL_DATA_X1: u8 = 0x1F; // Registro de inicio de datos acelerómetro

pub struct AccelerationModule {
    // Almacena: [gx, gy, gz, ax, ay, az]
    data: Arc<Mutex<[f32; 6]>>,
}

impl AccelerationModule {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new([0.0; 6])),
        }
    }

    pub fn init(&self) {
        logger("ACCELERATION", "Inicializando sensor de inercia vía SPI...");
    }

    pub fn run(&self) {
        let data_ptr = Arc::clone(&self.data);
        
        // Muestreo interno a 25ms (40Hz) según tu requerimiento
        let period = Duration::from_millis(25);

        thread::spawn(move || {
            // Configuración del bus SPI
            let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)
                .expect("ERROR: No se pudo inicializar SPI0. ¿Está habilitado en raspi-config?");

            // Setup inicial del sensor
            let _ = spi.write(&[REG_PWR_MGMT0, 0x0F]); // Power on
            let _ = spi.write(&[REG_GYRO_CONFIG0, 0x46]); // Config específica (basada en tu código anterior)
            let _ = spi.write(&[REG_ACCEL_CONFIG0, 0x46]);

            // Factores de conversión (ajustados según tu lógica vieja)
            let g_res = (2000.0 / 8.0) / 32768.0; 
            let a_res = 8.0 / 32768.0;

            logger("ACCELERATION", "Hilo de muestreo iniciado (25ms)");

            loop {
                let start = Instant::now();

                // Lectura de ráfaga (Burst read) de 12 bytes: 6 para Gyro, 6 para Accel
                let mut read_buf = [0u8; 13];
                let mut cmd = [0u8; 13];
                cmd[0] = REG_ACCEL_DATA_X1 | 0x80; // Dirección base con bit de lectura

                if spi.transfer(&mut read_buf, &cmd).is_ok() {
                    let raw = &read_buf[1..];

                    let to_f32 = |msb: u8, lsb: u8| {
                        ((((msb as u16) << 8) | lsb as u16) as i16) as f32
                    };

                    // Mapeo: gx, gy, gz (0-5) | ax, ay, az (6-11)
                    let sample = [
                        to_f32(raw[0], raw[1]) * g_res,
                        to_f32(raw[2], raw[3]) * g_res,
                        to_f32(raw[4], raw[5]) * g_res,
                        to_f32(raw[6], raw[7]) * a_res,
                        to_f32(raw[8], raw[9]) * a_res,
                        to_f32(raw[10], raw[11]) * a_res,
                    ];

                    if let Ok(mut guard) = data_ptr.lock() {
                        *guard = sample;
                    }
                }

                // Control de tiempo para los 25ms constantes
                let elapsed = start.elapsed();
                if elapsed < period {
                    thread::sleep(period - elapsed);
                }
            }
        });
    }

    /// El "Getter" que usará el Bridge cada 50ms
    pub fn get_latest(&self) -> [f32; 6] {
        *self.data.lock().unwrap_or_else(|e| e.into_inner())
    }
}