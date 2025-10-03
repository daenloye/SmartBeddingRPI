#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdio.h>
#include <fcntl.h>
#include <linux/i2c-dev.h>
#include <sys/ioctl.h>
#include <gpiod.h>

#define ROWS 16
#define COLS 12

// Shift register GPIO BCM
#define DATA_PIN 5
#define SHIFT_CLOCK_PIN 13
#define LATCH_CLOCK_PIN 6

// Direcciones I2C
#define MCP23017_ADDR 0x21
#define ADS1015_ADDR 0x48

// Filas y columnas
uint16_t rowArray[ROWS] = {
    0b1000000000000000,0b0100000000000000,0b0010000000000000,0b0001000000000000,
    0b0000100000000000,0b0000010000000000,0b0000001000000000,0b0000000100000000,
    0b0000000010000000,0b0000000001000000,0b0000000000100000,0b0000000000010000,
    0b0000000000001000,0b0000000000000100,0b0000000000000010,0b0000000000000001
};

uint8_t colArray[COLS] = {0,1,2,3,4,5,6,7,8,9,10,11};

// GPIO
struct gpiod_chip *chip;
struct gpiod_line *line_data, *line_clock, *line_latch;

// I2C
int i2c_fd_mcp, i2c_fd_ads;

// --------------------- GPIO shift register ----------------------
void init_gpio() {
    chip = gpiod_chip_open_by_name("gpiochip0");
    line_data = gpiod_chip_get_line(chip, DATA_PIN);
    line_clock = gpiod_chip_get_line(chip, SHIFT_CLOCK_PIN);
    line_latch = gpiod_chip_get_line(chip, LATCH_CLOCK_PIN);

    gpiod_line_request_output(line_data, "matrix", 0);
    gpiod_line_request_output(line_clock, "matrix", 0);
    gpiod_line_request_output(line_latch, "matrix", 0);
}

void shift_register_out(uint16_t val) {
    for(int i=15;i>=0;i--){
        int bit = (val>>i)&1;
        gpiod_line_set_value(line_data,bit);
        gpiod_line_set_value(line_clock,1);
        usleep(10);
        gpiod_line_set_value(line_clock,0);
    }
    gpiod_line_set_value(line_latch,1);
    usleep(10);
    gpiod_line_set_value(line_latch,0);
}

// --------------------- MCP23017 ----------------------
void init_mcp23017() {
    i2c_fd_mcp = open("/dev/i2c-1", O_RDWR);
    ioctl(i2c_fd_mcp, I2C_SLAVE, MCP23017_ADDR);

    uint8_t buf[3];

    // Config GPIOA (0-7) salida
    buf[0] = 0x00; buf[1] = 0x00; write(i2c_fd_mcp, buf, 2);
    // Config GPIOB (0-7) salida
    buf[0] = 0x01; buf[1] = 0x00; write(i2c_fd_mcp, buf, 2);
}

void set_column(uint8_t col) {
    uint8_t buf[2];
    buf[0] = 0x12; // GPIOA
    buf[1] = 1<<col; // activar solo el bit de la columna
    write(i2c_fd_mcp, buf, 2);

    // Enable en GPIOB bit0
    buf[0] = 0x13; buf[1] = 1; write(i2c_fd_mcp, buf, 2);
}

// --------------------- ADS1015 ----------------------
void init_ads1015() {
    i2c_fd_ads = open("/dev/i2c-1", O_RDWR);
    ioctl(i2c_fd_ads, I2C_SLAVE, ADS1015_ADDR);
}

// Leer un canal del ADS1015
uint16_t read_adc_channel(uint8_t channel) {
    if(channel>3) return 0;
    uint16_t config = 0x8400; // single-shot, AINx-GND
    config |= (channel&0x03)<<12;

    uint8_t buf[3];
    buf[0]=0x01; // registro config
    buf[1]=(config>>8)&0xFF;
    buf[2]=config&0xFF;
    write(i2c_fd_ads, buf, 3);

    usleep(1500);

    buf[0]=0x00;
    write(i2c_fd_ads, buf, 1);

    uint8_t readbuf[2];
    read(i2c_fd_ads, readbuf, 2);
    return ((readbuf[0]<<8)|readbuf[1])>>4;
}

// --------------------- Inicialización ----------------------
void init_matrix() {
    srand(time(NULL));
    init_gpio();
    init_mcp23017();
    init_ads1015();
}

// --------------------- Muestreo matriz ----------------------
void sample_matrix(uint16_t *buffer) {
    for(int i=0;i<ROWS;i++){
        shift_register_out(rowArray[i]);
        for(int j=0;j<COLS;j++){
            set_column(colArray[j]);
            usleep(1000);
            buffer[i*COLS+j] = read_adc_channel(j%4); // ADS1015 tiene 4 canales
        }
    }
}
