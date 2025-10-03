#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <fcntl.h>
#include <linux/i2c-dev.h>
#include <i2c/smbus.h>
#include <gpiod.h>
#include <stdlib.h>

#define ROW_SIZE 16
#define COL_SIZE 12

#define DATA_PIN 5
#define SHIFT_CLOCK_PIN 13
#define LATCH_CLOCK_PIN 6

#define MCP23017_ADDR 0x21
#define ADS1015_ADDR  0x48

static struct gpiod_chip *chip;
static struct gpiod_line *data_line, *clk_line, *latch_line;

static int i2c_fd_mcp = -1;
static int i2c_fd_ads = -1;

uint16_t rowArray[ROW_SIZE] = {
    0b1000000000000000,0b0100000000000000,0b0010000000000000,0b0001000000000000,
    0b0000100000000000,0b0000010000000000,0b0000001000000000,0b0000000100000000,
    0b0000000010000000,0b0000000001000000,0b0000000000100000,0b0000000000010000,
    0b0000000000001000,0b0000000000000100,0b0000000000000010,0b0000000000000001
};

uint8_t colArray[COL_SIZE] = {
    0b00010000,0b00010001,0b00010010,0b00010011,
    0b00010100,0b00010101,0b00010110,0b00010111,
    0b00011000,0b00011001,0b00011010,0b00011011
};

int matrix_init() {
    // GPIO
    chip = gpiod_chip_open_by_name("gpiochip0");
    if (!chip) return -1;

    data_line = gpiod_chip_get_line(chip, DATA_PIN);
    clk_line  = gpiod_chip_get_line(chip, SHIFT_CLOCK_PIN);
    latch_line= gpiod_chip_get_line(chip, LATCH_CLOCK_PIN);

    if (!data_line || !clk_line || !latch_line) return -2;

    gpiod_line_request_output(data_line, "matrix", 0);
    gpiod_line_request_output(clk_line,  "matrix", 0);
    gpiod_line_request_output(latch_line,"matrix", 0);

    // I2C MCP23017
    if ((i2c_fd_mcp = open("/dev/i2c-1", O_RDWR)) < 0) return -3;
    if (ioctl(i2c_fd_mcp, I2C_SLAVE, MCP23017_ADDR) < 0) return -4;
    i2c_smbus_write_byte_data(i2c_fd_mcp, 0x00, 0x00); // IODIRA
    i2c_smbus_write_byte_data(i2c_fd_mcp, 0x01, 0x00); // IODIRB

    // I2C ADS1015
    if ((i2c_fd_ads = open("/dev/i2c-1", O_RDWR)) < 0) return -5;
    if (ioctl(i2c_fd_ads, I2C_SLAVE, ADS1015_ADDR) < 0) return -6;

    return 0;
}

void shift_register_out(uint16_t val){
    for(int i=15;i>=0;i--){
        int bit = (val >> i) & 1;
        gpiod_line_set_value(data_line, bit);
        gpiod_line_set_value(clk_line,1);
        usleep(10);
        gpiod_line_set_value(clk_line,0);
    }
    gpiod_line_set_value(latch_line,1);
    usleep(10);
    gpiod_line_set_value(latch_line,0);
}

void set_column(uint8_t col){
    i2c_smbus_write_byte(i2c_fd_mcp, col);
}

uint16_t read_adc(){
    // Configurar single-shot AIN0 vs GND, PGA ±4.096V, 1600SPS
    i2c_smbus_write_word_data(i2c_fd_ads, 0x01, 0xC383);
    usleep(1200); // espera conversión ~1.2ms
    uint16_t raw = i2c_smbus_read_word_data(i2c_fd_ads,0x00);
    uint16_t value = (raw << 8) | (raw >> 8); // swap bytes
    return value;
}

void matrix_update(uint16_t* buffer){
    for(int i=0;i<ROW_SIZE;i++){
        shift_register_out(rowArray[i]);
        gpiod_line_set_value(latch_line,1);
        gpiod_line_set_value(latch_line,0);
        for(int j=0;j<COL_SIZE;j++){
            set_column(colArray[j]);
            usleep(1000);
            buffer[i*COL_SIZE+j] = read_adc();
        }
    }
}
