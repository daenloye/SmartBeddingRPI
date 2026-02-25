use rppal::gpio::{Gpio, OutputPin};
use rppal::i2c::I2c;
use std::thread;
use std::time::Duration;

pub const ROW_SIZE: usize = 16;
pub const COL_SIZE: usize = 12;

const MCP23017_ADDR: u16 = 0x21;
const ADS1015_ADDR: u16 = 0x48;

pub struct PressureMatrix {
    data_pin: OutputPin,
    clk_pin: OutputPin,
    latch_pin: OutputPin,
    i2c: I2c,
    row_array: [u16; ROW_SIZE],
    col_array: [u8; COL_SIZE],
}

impl PressureMatrix {
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let gpio = Gpio::new()?;
        let mut i2c = I2c::new()?;

        // Configurar MCP23017 (como en tu C: IODIRA y IODIRB)
        i2c.set_slave_address(MCP23017_ADDR)?;
        i2c.smbus_write_byte(0x00, 0xE0)?; // IODIRA
        i2c.smbus_write_byte(0x01, 0xFF)?; // IODIRB

        Ok(Self {
            data_pin: gpio.get(5)?.into_output(),
            clk_pin: gpio.get(13)?.into_output(),
            latch_pin: gpio.get(6)?.into_output(),
            i2c,
            row_array: [
                0b1000000000000000, 0b0100000000000000, 0b0010000000000000, 0b0001000000000000,
                0b0000100000000000, 0b0000010000000000, 0b0000001000000000, 0b0000000100000000,
                0b0000000010000000, 0b0000000001000000, 0b0000000000100000, 0b0000000000010000,
                0b0000000000001000, 0b0000000000000100, 0b0000000000000010, 0b0000000000000001,
            ],
            col_array: [
                0b00010000, 0b00010001, 0b00010010, 0b00010011,
                0b00010100, 0b00010101, 0b00010110, 0b00010111,
                0b00011000, 0b00011001, 0b00011010, 0b00011011,
            ],
        })
    }

    fn shift_out(&mut self, val: u16) {
        for i in (0..16).rev() {
            if (val >> i) & 1 == 1 { self.data_pin.set_high(); } else { self.data_pin.set_low(); }
            self.clk_pin.set_high();
            self.clk_pin.set_low();
        }
        self.latch_pin.set_high();
        self.latch_pin.set_low();
    }

    fn set_column(&mut self, col_val: u8) -> Result<(), rppal::i2c::Error> {
        self.i2c.set_slave_address(MCP23017_ADDR)?;
        let addr = col_val & 0x0F;
        let enable = (col_val >> 4) & 0x01;
        let olat_a = (enable << 4) | addr;
        self.i2c.smbus_write_byte(0x14, olat_a) // 0x14 es MCP_OLATA
    }

    fn read_adc(&mut self) -> u16 {
        let _ = self.i2c.set_slave_address(ADS1015_ADDR);
        let config: u16 = 0x8583; // Tu config simplificada en un solo valor
        let _ = self.i2c.smbus_write_word(0x01, config.swap_bytes());
        
        thread::sleep(Duration::from_micros(500));
        
        match self.i2c.smbus_read_word(0x00) {
            Ok(val) => {
                let raw = val.swap_bytes() >> 4;
                let value = raw & 0x0FFF;
                if value > 4000 { 0 } else { value * 35 }
            },
            Err(_) => 0,
        }
    }

    pub fn scan(&mut self) -> [[u16; COL_SIZE]; ROW_SIZE] {
        let mut matrix = [[0u16; COL_SIZE]; ROW_SIZE];
        for i in 0..ROW_SIZE {
            self.shift_out(self.row_array[i]);
            thread::sleep(Duration::from_millis(2));

            for j in 0..COL_SIZE {
                let _ = self.set_column(self.col_array[j]);
                thread::sleep(Duration::from_millis(1));
                matrix[i][j] = self.read_adc();
            }
        }
        matrix
    }
}