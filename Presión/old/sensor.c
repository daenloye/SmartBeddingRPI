// sensor.c
#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <time.h>
#include <gpiod.h>
#include <linux/i2c-dev.h>
#include <sys/ioctl.h>
#include <fcntl.h>
#include <errno.h>
#include <string.h>

#define MAX_ROWS 16
#define MAX_COLS 12

// GPIO shift register pins
#define DATA_PIN  5
#define SHCP_PIN 13
#define STCP_PIN  6

// I2C addresses
#define MCP23017_ADDR 0x21
#define ADS1015_ADDR  0x48

// ----------------- Globals -----------------
static int fd_mcp = -1;
static int fd_ads = -1;
static struct gpiod_chip *chip = NULL;
static struct gpiod_line *data_line = NULL;
static struct gpiod_line *shcp_line = NULL;
static struct gpiod_line *stcp_line = NULL;
static int initialized = 0;

// Pines MCP23017: A0-A3 para direccion, A4 como enable
static uint8_t col_addr[4] = {0,1,2,3};
static uint8_t enable_pin = 4;

// ----------------- Aux functions -----------------
static int init_gpio() {
    chip = gpiod_chip_open_by_name("gpiochip0");
    if (!chip) {
        perror("Error abriendo gpiochip0");
        return -1;
    }

    data_line = gpiod_chip_get_line(chip, DATA_PIN);
    shcp_line = gpiod_chip_get_line(chip, SHCP_PIN);
    stcp_line = gpiod_chip_get_line(chip, STCP_PIN);

    if (!data_line || !shcp_line || !stcp_line) {
        fprintf(stderr, "Error obteniendo líneas GPIO\n");
        return -1;
    }

    if (gpiod_line_request_output(data_line, "sensor", 0) < 0 ||
        gpiod_line_request_output(shcp_line, "sensor", 0) < 0 ||
        gpiod_line_request_output(stcp_line, "sensor", 0) < 0) {
        perror("Error solicitando líneas GPIO como salida");
        return -1;
    }

    printf("GPIO inicializado correctamente\n");
    return 0;
}

static int init_i2c() {
    fd_mcp = open("/dev/i2c-1", O_RDWR);
    if (fd_mcp < 0) { perror("Error abriendo I2C MCP23017"); return -1; }
    if (ioctl(fd_mcp, I2C_SLAVE, MCP23017_ADDR) < 0) { perror("Error dirección MCP23017"); return -1; }

    fd_ads = open("/dev/i2c-1", O_RDWR);
    if (fd_ads < 0) { perror("Error abriendo I2C ADS1015"); return -1; }
    if (ioctl(fd_ads, I2C_SLAVE, ADS1015_ADDR) < 0) { perror("Error dirección ADS1015"); return -1; }

    printf("I2C inicializado correctamente\n");
    return 0;
}

static void shift_register_out(uint16_t val) {
    for (int i = 15; i >= 0; i--) {
        int bit = (val >> i) & 1;
        gpiod_line_set_value(data_line, bit);
        gpiod_line_set_value(shcp_line, 1);
        gpiod_line_set_value(shcp_line, 0);
    }
    gpiod_line_set_value(stcp_line, 1);
    gpiod_line_set_value(stcp_line, 0);
}

static void set_column(int col_index) {
    // A0-A3
    for(int i=0;i<4;i++) {
        uint8_t bit = (col_index >> i) & 1;
        uint8_t buf[2] = {0x12, 0}; // GPIOA
        if(i == 0) buf[1] |= bit << 0;
        if(i == 1) buf[1] |= bit << 1;
        if(i == 2) buf[1] |= bit << 2;
        if(i == 3) buf[1] |= bit << 3;
        write(fd_mcp, buf, 2);
    }
    // Enable pin (A4) activo
    uint8_t buf[2] = {0x12, 1 << enable_pin};
    write(fd_mcp, buf, 2);
}

static int read_ads1015() {
    uint8_t config[3] = {0x01, 0xC3, 0x83}; // Siempre canal 0
    if(write(fd_ads, config, 3)!=3){ perror("Config ADS"); return -1; }
    usleep(20000); // 20ms para estabilidad
    uint8_t read_buf[2];
    if(read(fd_ads, read_buf, 2)!=2){ perror("Read ADS"); return -1; }
    int val = ((read_buf[0]<<8)|read_buf[1])>>4;
    return val;
}

// ----------------- Main -----------------
int main() {
    if(init_gpio()<0 || init_i2c()<0) return -1;

    double matrix[MAX_ROWS][MAX_COLS];

    uint16_t rowArray[MAX_ROWS] = {
        0b1000000000000000,0b0100000000000000,0b0010000000000000,0b0001000000000000,
        0b0000100000000000,0b0000010000000000,0b0000001000000000,0b0000000100000000,
        0b0000000010000000,0b0000000001000000,0b0000000000100000,0b0000000000010000,
        0b0000000000001000,0b0000000000000100,0b0000000000000010,0b0000000000000001
    };

    for(int r=0;r<MAX_ROWS;r++){
        shift_register_out(rowArray[r]);
        for(int c=0;c<MAX_COLS;c++){
            set_column(c);
            usleep(20000); // 20 ms para estabilización
            int val = read_ads1015();
            matrix[r][c] = val;
            printf("Fila %2d, Col %2d = %4d\n", r, c, val);
        }
    }
    return 0;
}
