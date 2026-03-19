use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::utils::logger;

// Direcciones de registros ICM-42605
const REG_TEMP_DATA: u8 = 0x1D;
const REG_PWR_MGMT0: u8 = 0x4E;
const REG_GYRO_CONFIG0: u8 = 0x4F;
const REG_ACCEL_CONFIG0: u8 = 0x50;

// Configuración de bits para 250dps, 2G y 1000Hz ODR
const GFS_250DPS: u8 = 0x03;
const AFS_2G: u8 = 0x03;
const GODR_1000HZ: u8 = 0x06;
const AODR_1000HZ: u8 = 0x06;

pub struct AccelerationModule {
    data: Arc<Mutex<[f32; 6]>>,
}

impl AccelerationModule {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new([0.0; 6])),
        }
    }

    pub fn init(&self) {
        logger("ACCELERATION", "Inicializando sensor de inercia...");
    }

    pub fn run(&self) {
        let data_ptr = Arc::clone(&self.data);
        let period = Duration::from_millis(25); // Muestreo interno a 40Hz

        thread::spawn(move || {
            let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)
                .expect("ERROR: No se pudo inicializar SPI0");

            // 1. Despertar el sensor
            let _ = spi.write(&[REG_PWR_MGMT0, 0x0F]);
            thread::sleep(Duration::from_millis(15)); // Tiempo para que el PLL estabilice

            // 2. Configuración de sensibilidad y tasa de datos
            let _ = spi.write(&[REG_GYRO_CONFIG0, (GFS_250DPS << 5) | GODR_1000HZ]);
            let _ = spi.write(&[REG_ACCEL_CONFIG0, (AFS_2G << 5) | AODR_1000HZ]);

            // 3. Factores de conversión basados en la configuración anterior
            let g_res = (2000.0 / f32::powi(2.0, GFS_250DPS as i32)) / 32768.0;
            let a_res = f32::powi(2.0, (AFS_2G + 1) as i32) / 32768.0;

            logger("ACCELERATION", "Bucle de captura iniciado.");

            loop {
                let start = Instant::now();

                // Comando de lectura: 1 byte cmd + 14 bytes (2 Temp + 6 Gyro + 6 Accel)
                let mut read_buf = [0u8; 15];
                let mut cmd = [0u8; 15];
                cmd[0] = REG_TEMP_DATA | 0x80; // Leer desde 0x1D

                if spi.transfer(&mut read_buf, &cmd).is_ok() {
                    // Ignoramos el primer byte del buffer (basura SPI)
                    let raw = &read_buf[1..];

                    let to_i16 = |msb: u8, lsb: u8| {
                        (((msb as u16) << 8) | lsb as u16) as i16
                    };

                    // raw[0..1] -> Temperatura
                    // raw[2..7] -> Gyro (X, Y, Z)
                    // raw[8..13] -> Accel (X, Y, Z)
                    let sample = [
                        to_i16(raw[2], raw[3]) as f32 * g_res,
                        to_i16(raw[4], raw[5]) as f32 * g_res,
                        to_i16(raw[6], raw[7]) as f32 * g_res,
                        to_i16(raw[8], raw[9]) as f32 * a_res,
                        to_i16(raw[10], raw[11]) as f32 * a_res,
                        to_i16(raw[12], raw[13]) as f32 * a_res,
                    ];

                    if let Ok(mut guard) = data_ptr.lock() {
                        *guard = sample;
                    }
                }

                let elapsed = start.elapsed();
                if elapsed < period {
                    thread::sleep(period - elapsed);
                }
            }
        });
    }

    pub fn get_latest(&self) -> [f32; 6] {
        // Corrección del error E0308: 
        // Obtenemos el guardián y devolvemos una copia del valor interno.
        let guard = self.data.lock().unwrap_or_else(|e| e.into_inner());
        *guard
    }
}