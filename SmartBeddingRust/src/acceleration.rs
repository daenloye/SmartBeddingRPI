use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// Importamos la configuración
use crate::config::CONFIG;

const REG_TEMP_DATA: u8 = 0x1D;
const REG_PWR_MGMT0: u8 = 0x4E;
const REG_GYRO_CONFIG0: u8 = 0x4F;
const REG_ACCEL_CONFIG0: u8 = 0x50;

const GFS_250DPS: u8 = 0x03;
const AFS_2G: u8 = 0x03;
const GODR_1000HZ: u8 = 0x06;
const AODR_1000HZ: u8 = 0x06;

pub struct AccelerationModule {
    data: Arc<Mutex<[f32; 6]>>,
    running: Arc<Mutex<bool>>,
}

impl AccelerationModule {
    pub fn new(bus: Bus, slave: SlaveSelect) -> Self {
        let data = Arc::new(Mutex::new([0.0; 6]));
        let running = Arc::new(Mutex::new(true));
        
        let data_clone = Arc::clone(&data);
        let running_clone = Arc::clone(&running);

        thread::spawn(move || {
            let mut spi = Spi::new(bus, slave, 1_000_000, Mode::Mode0)
                .expect("Error SPI: ¿Está activado el bus en la Pi Zero?");

            // Sensibilidad basada en bits de configuración
            let g_res = (2000.0 / f32::powi(2.0, GFS_250DPS as i32)) / 32768.0;
            let a_res = f32::powi(2.0, (AFS_2G + 1) as i32) / 32768.0;

            // Setup inicial (Power On + Config)
            let _ = spi.write(&[REG_PWR_MGMT0, 0x0F]);
            let _ = spi.write(&[REG_GYRO_CONFIG0, (GFS_250DPS << 5) | GODR_1000HZ]);
            let _ = spi.write(&[REG_ACCEL_CONFIG0, (AFS_2G << 5) | AODR_1000HZ]);

            // Muestreo a la tasa definida por acceleration_period_ms (40Hz)
            let period = Duration::from_millis(CONFIG.acceleration_period_ms);

            while *running_clone.lock().unwrap() {
                let start = Instant::now();

                let mut read_buf = [0u8; 15];
                let mut cmd = [0u8; 15];
                cmd[0] = REG_TEMP_DATA | 0x80;

                if spi.transfer(&mut read_buf, &cmd).is_ok() {
                    let raw = &read_buf[1..];

                    let to_f32 = |msb: u8, lsb: u8| {
                        ((((msb as u16) << 8) | lsb as u16) as i16) as f32
                    };

                    // gx, gy, gz, ax, ay, az
                    let sample = [
                        to_f32(raw[2], raw[3]) * g_res,
                        to_f32(raw[4], raw[5]) * g_res,
                        to_f32(raw[6], raw[7]) * g_res,
                        to_f32(raw[8], raw[9]) * a_res,
                        to_f32(raw[10], raw[11]) * a_res,
                        to_f32(raw[12], raw[13]) * a_res,
                    ];

                    if let Ok(mut guard) = data_clone.lock() {
                        *guard = sample;
                    }
                }

                let elapsed = start.elapsed();
                if elapsed < period {
                    thread::sleep(period - elapsed);
                }
            }
        });

        Self { data, running }
    }

    pub fn get_latest_data(&self) -> [f32; 6] {
        *self.data.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn stop(&self) {
        if let Ok(mut r) = self.running.lock() {
            *r = false;
        }
    }
}