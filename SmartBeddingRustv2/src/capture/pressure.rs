use rppal::gpio::{Gpio, OutputPin};
use rppal::i2c::I2c;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use crate::utils::logger;

pub const ROW_SIZE: usize = 16;
pub const COL_SIZE: usize = 12;

const MCP23017_ADDR: u16 = 0x21;
const ADS1015_ADDR: u16 = 0x48;

pub struct PressureModule {
    i2c: Option<Arc<Mutex<I2c>>>,
    pub current_matrix: Arc<Mutex<[[u16; COL_SIZE]; ROW_SIZE]>>,
    // Guardamos los pines temporalmente tras inicializarlos en el hilo principal
    pins: Arc<Mutex<Option<(OutputPin, OutputPin, OutputPin)>>>,
}

impl PressureModule {
    pub fn new() -> Self {
        Self {
            i2c: None,
            current_matrix: Arc::new(Mutex::new([[0u16; COL_SIZE]; ROW_SIZE])),
            pins: Arc::new(Mutex::new(None)),
        }
    }

    pub fn init(&mut self, shared_i2c: Arc<Mutex<I2c>>) {
        logger("PRESSURE", "Iniciando configuración de matriz de presión (16x12)...");
        
        // 1. Inicializar el expansor MCP23017 una sola vez
        if let Ok(mut i2c) = shared_i2c.lock() {
            let _ = i2c.set_slave_address(MCP23017_ADDR);
            let _ = i2c.smbus_write_byte(0x00, 0xE0); 
            let _ = i2c.smbus_write_byte(0x01, 0xFF);
        }

        self.i2c = Some(shared_i2c);

        // 2. Inicializar GPIO en el HILO PRINCIPAL (Igual que en tu código original)
        // Usamos un match para no hacer panic si hay un error de permisos
        match Gpio::new() {
            Ok(gpio) => {
                let data_pin = gpio.get(5).expect("Error obteniendo pin 5").into_output();
                let clk_pin = gpio.get(13).expect("Error obteniendo pin 13").into_output();
                let latch_pin = gpio.get(6).expect("Error obteniendo pin 6").into_output();
                
                *self.pins.lock().unwrap() = Some((data_pin, clk_pin, latch_pin));
                logger("PRESSURE", "GPIOs del Shift Register configurados correctamente.");
            },
            Err(e) => {
                logger("PRESSURE", &format!("ERROR CRÍTICO inicializando GPIO: {:?}", e));
                logger("PRESSURE", "Asegúrate de ejecutar el programa con permisos suficientes.");
            }
        }
    }

    pub fn run(&self) {
        let i2c_ptr = self.i2c.as_ref().expect("I2C no vinculado en Pressure").clone();
        let matrix_ptr = self.current_matrix.clone();

        // 3. Extraemos los pines para moverlos al hilo. 
        // Si GPIO falló en init(), detenemos el escaneo sin causar un panic global.
        let pins_opt = self.pins.lock().unwrap().take();
        if pins_opt.is_none() {
            logger("PRESSURE", "Abortando inicio del hilo: pines GPIO no disponibles.");
            return;
        }

        thread::spawn(move || {
            let (mut data_pin, mut clk_pin, mut latch_pin) = pins_opt.unwrap();

            let row_array: [u16; 16] = [
                0b1000000000000000, 0b0100000000000000, 0b0010000000000000, 0b0001000000000000,
                0b0000100000000000, 0b0000010000000000, 0b0000001000000000, 0b0000000100000000,
                0b0000000010000000, 0b0000000001000000, 0b0000000000100000, 0b0000000000010000,
                0b0000000000001000, 0b0000000000000100, 0b0000000000000010, 0b0000000000000001,
            ];

            let col_array: [u8; 12] = [
                0b00010000, 0b00010001, 0b00010010, 0b00010011,
                0b00010100, 0b00010101, 0b00010110, 0b00010111,
                0b00011000, 0b00011001, 0b00011010, 0b00011011,
            ];

            logger("PRESSURE", "Hilo de escaneo continuo iniciado.");

            loop {
                let mut working_buffer = [[0u16; COL_SIZE]; ROW_SIZE];

                for i in 0..ROW_SIZE {
                    // 1. Activar Fila (Shift Out)
                    Self::shift_out(&mut data_pin, &mut clk_pin, &mut latch_pin, row_array[i]);
                    thread::sleep(Duration::from_millis(8)); // TIEMPO CRÍTICO

                    for j in 0..COL_SIZE {
                        // 2. Activar Columna (MCP23017)
                        let _ = Self::set_column(&i2c_ptr, col_array[j]);
                        thread::sleep(Duration::from_millis(1)); // TIEMPO CRÍTICO

                        // 3. Leer ADC (ADS1015)
                        working_buffer[i][j] = Self::read_adc(&i2c_ptr);
                    }
                }

                // 4. Actualizar la matriz compartida
                if let Ok(mut guard) = matrix_ptr.lock() {
                    *guard = working_buffer;
                }
            }
        });
    }

    fn shift_out(data: &mut OutputPin, clk: &mut OutputPin, latch: &mut OutputPin, val: u16) {
        for i in (0..16).rev() {
            if (val >> i) & 1 == 1 { data.set_high(); } else { data.set_low(); }
            clk.set_high();
            clk.set_low();
        }
        latch.set_high();
        latch.set_low();
    }

    fn set_column(i2c_mutex: &Arc<Mutex<I2c>>, col_val: u8) -> Result<(), String> {
        let mut i2c = i2c_mutex.lock().map_err(|_| "Mutex poisoned")?;
        i2c.set_slave_address(MCP23017_ADDR).map_err(|e| e.to_string())?;
        let addr = col_val & 0x0F;
        let enable = (col_val >> 4) & 0x01;
        let olat_a = (enable << 4) | addr;
        i2c.smbus_write_byte(0x14, olat_a).map_err(|e| e.to_string())
    }

    fn read_adc(i2c_mutex: &Arc<Mutex<I2c>>) -> u16 {
        if let Ok(mut i2c) = i2c_mutex.lock() {
            let _ = i2c.set_slave_address(ADS1015_ADDR);
            let config: u16 = 0x8583; 
            let _ = i2c.smbus_write_word(0x01, config.swap_bytes());
            
            drop(i2c); // Soltamos el bus para que otros hilos respiren
            thread::sleep(Duration::from_micros(800)); // TIEMPO CRÍTICO
            
            if let Ok(mut i2c_locked) = i2c_mutex.lock() {
                let _ = i2c_locked.set_slave_address(ADS1015_ADDR);
                // Exactamente el mismo manejo de Result que en tu código funcional
                match i2c_locked.smbus_read_word(0x00) {
                    Ok(val) => {
                        let raw = val.swap_bytes() >> 4;
                        let value = raw & 0x0FFF;
                        return if value > 4000 { 0 } else { value * 35 };
                    },
                    Err(_) => return 0,
                }
            }
        }
        0
    }

    pub fn get_latest(&self) -> [[u16; COL_SIZE]; ROW_SIZE] {
        *self.current_matrix.lock().unwrap_or_else(|e| e.into_inner())
    }
}