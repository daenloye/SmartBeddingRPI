#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdlib.h>
#include <linux/i2c-dev.h>
#include <i2c/smbus.h>
#include <gpiod.h>
#include <time.h>
#include <string.h>
#include <sys/ioctl.h>

#define ROW_SIZE 16
#define COL_SIZE 12

// Pines GPIO BCM
#define DATA_PIN 5
#define SHIFT_CLOCK_PIN 13
#define LATCH_CLOCK_PIN 6

// I2C
#define MCP23017_ADDR 0x21
#define ADS1015_ADDR  0x48

// MCP23017 registers
#define MCP_IODIRA   0x00
#define MCP_IODIRB   0x01
#define MCP_OLATA    0x14
#define MCP_OLATB    0x15

// ADS1015 registers
#define ADS_CONVERSION 0x00
#define ADS_CONFIG     0x01

// ADS1015 Config bits
#define ADS_OS_SINGLE  0x8000  // Start single conversion
#define ADS_MUX_AIN0   0x4000  // AIN0 vs GND
#define ADS_PGA_4_096V 0x0200  // ±4.096V range
#define ADS_MODE_SINGLE 0x0100 // Single-shot mode
#define ADS_DR_1600SPS 0x0080  // 1600 samples/sec
#define ADS_COMP_QUE_DISABLE 0x0003  // Disable comparator

// --------------- Globales ---------------
static struct gpiod_chip *chip = NULL;
static struct gpiod_line *data_line = NULL, *clk_line = NULL, *latch_line = NULL;
static int i2c_fd = -1;
static int current_i2c_addr = -1;

// Arrays de fila y columna
const uint16_t rowArray[ROW_SIZE] = {
    0b1000000000000000,0b0100000000000000,0b0010000000000000,0b0001000000000000,
    0b0000100000000000,0b0000010000000000,0b0000001000000000,0b0000000100000000,
    0b0000000010000000,0b0000000001000000,0b0000000000100000,0b0000000000010000,
    0b0000000000001000,0b0000000000000100,0b0000000000000010,0b0000000000000001
};

const uint8_t colArray[COL_SIZE] = {
    0b00010000,0b00010001,0b00010010,0b00010011,
    0b00010100,0b00010101,0b00010110,0b00010111,
    0b00011000,0b00011001,0b00011010,0b00011011
};

// ----------------- I2C Device Selection -----------------
static inline void select_i2c_device(int addr) {
    if (current_i2c_addr != addr) {
        ioctl(i2c_fd, I2C_SLAVE, addr);
        current_i2c_addr = addr;
    }
}

// ----------------- Shift Register -----------------
static inline void shift_register_out(uint16_t val) {
    for (int i = 15; i >= 0; --i) {
        int bit = (val >> i) & 0x1;
        gpiod_line_set_value(data_line, bit);
        gpiod_line_set_value(clk_line, 1);
        gpiod_line_set_value(clk_line, 0);
    }
    gpiod_line_set_value(latch_line, 1);
    gpiod_line_set_value(latch_line, 0);
}

// ----------------- MCP23017: set_column -----------------
static inline void set_column(uint8_t col_val) {
    select_i2c_device(MCP23017_ADDR);
    
    uint8_t addr = col_val & 0x0F;
    uint8_t enable = (col_val >> 4) & 0x01;
    uint8_t olat_a = (enable << 4) | addr;
    
    i2c_smbus_write_byte_data(i2c_fd, MCP_OLATA, olat_a);
}

// ----------------- ADS1015: read_adc (CORREGIDO) -----------------
static inline uint16_t read_adc() {
    select_i2c_device(ADS1015_ADDR);
    
    // Configuración completa para ADS1015
    uint16_t config = ADS_OS_SINGLE |      // Start conversion
                      ADS_MUX_AIN0 |        // AIN0 vs GND
                      ADS_PGA_4_096V |      // ±4.096V
                      ADS_MODE_SINGLE |     // Single-shot
                      ADS_DR_1600SPS |      // 1600 SPS
                      ADS_COMP_QUE_DISABLE; // Disable comparator
    
    // Escribir config
    if (i2c_smbus_write_word_data(i2c_fd, ADS_CONFIG, config) < 0) {
        return 0;
    }
    
    // Esperar a que la conversión termine
    for (int i = 0; i < 1000; ++i) { // max ~100ms
        int status = i2c_smbus_read_word_data(i2c_fd, ADS_CONFIG);
        if (status < 0) return 0;
        uint16_t status_word = ((status << 8) | (status >> 8));
        if (status_word & 0x8000) break; // OS=1 → conversión lista
        usleep(100); // 0.1ms
    }
    
    // Leer resultado
    int raw = i2c_smbus_read_word_data(i2c_fd, ADS_CONVERSION);
    if (raw < 0) return 0;
    
    uint16_t value = ((raw << 8) | (raw >> 8)) >> 4;
    value &= 0x0FFF; // 12 bits
    return value;
}

// ----------------- Inicialización -----------------
int matrix_init() {
    // ---- GPIO ----
    chip = gpiod_chip_open_by_name("gpiochip0");
    if (!chip) {
        perror("gpiod_chip_open");
        return -1;
    }
    
    data_line  = gpiod_chip_get_line(chip, DATA_PIN);
    clk_line   = gpiod_chip_get_line(chip, SHIFT_CLOCK_PIN);
    latch_line = gpiod_chip_get_line(chip, LATCH_CLOCK_PIN);
    
    if (!data_line || !clk_line || !latch_line) {
        fprintf(stderr, "Error obteniendo líneas GPIO\n");
        return -2;
    }
    
    if (gpiod_line_request_output(data_line, "matrix", 0) < 0 ||
        gpiod_line_request_output(clk_line, "matrix", 0) < 0 ||
        gpiod_line_request_output(latch_line, "matrix", 0) < 0) {
        perror("gpiod_line_request_output");
        return -3;
    }

    // ---- I2C ----
    if ((i2c_fd = open("/dev/i2c-1", O_RDWR)) < 0) {
        perror("open i2c");
        return -4;
    }
    
    // Configurar MCP23017
    current_i2c_addr = -1;
    select_i2c_device(MCP23017_ADDR);
    
    i2c_smbus_write_byte_data(i2c_fd, MCP_IODIRA, 0xE0);
    i2c_smbus_write_byte_data(i2c_fd, MCP_IODIRB, 0xFF);
    i2c_smbus_write_byte_data(i2c_fd, MCP_OLATA, 0x00);

    return 0;
}

// ----------------- Scan Matrix -----------------
void matrix_update(uint16_t matrix[ROW_SIZE][COL_SIZE]) {
    for (int i = 0; i < ROW_SIZE; ++i) {
        shift_register_out(rowArray[i]);
        
        gpiod_line_set_value(latch_line, 1);
        gpiod_line_set_value(latch_line, 0);
        
        // Delay extra para estabilizar fila
        usleep(2000); // 2ms

        for (int j = 0; j < COL_SIZE; ++j) {
            set_column(colArray[j]);
            usleep(1000); // 1ms para estabilizar columna
            matrix[i][j] = read_adc();
        }
    }
    
    // Cleanup
    select_i2c_device(MCP23017_ADDR);
    i2c_smbus_write_byte_data(i2c_fd, MCP_OLATA, 0x00);
    shift_register_out(0x0000);
}

// ----------------- Limpieza -----------------
void matrix_cleanup() {
    if (i2c_fd >= 0) {
        select_i2c_device(MCP23017_ADDR);
        i2c_smbus_write_byte_data(i2c_fd, MCP_OLATA, 0x00);
        close(i2c_fd);
    }
    if (chip) {
        shift_register_out(0x0000);
        gpiod_chip_close(chip);
    }
}

// ----------------- Main -----------------
int main() {
    printf("Inicializando hardware...\n");
    
    if (matrix_init() != 0) {
        fprintf(stderr, "Error inicializando hardware\n");
        matrix_cleanup();
        return 1;
    }
    
    printf("Hardware inicializado.\n");
    printf("Presiona Ctrl+C para salir.\n\n");
    
    uint16_t matrix[ROW_SIZE][COL_SIZE];
    memset(matrix, 0, sizeof(matrix));
    
    struct timespec start, end;
    int frame = 0;
    
    while (1) {
        clock_gettime(CLOCK_MONOTONIC, &start);
        
        matrix_update(matrix);
        
        clock_gettime(CLOCK_MONOTONIC, &end);
        double elapsed = (end.tv_sec - start.tv_sec) + 
                        (end.tv_nsec - start.tv_nsec) / 1e9;
        
        printf("\033[2J\033[H");
        printf("Frame: %d | Tiempo: %.3f s | FPS: %.1f\n\n", 
               frame++, elapsed, 1.0/elapsed);
        
        for (int i = 0; i < ROW_SIZE; ++i) {
            for (int j = 0; j < COL_SIZE; ++j) {
                if (matrix[i][j] > 100) {
                    printf("\033[1;32m%5u\033[0m ", matrix[i][j]);
                } else {
                    printf("%5u ", matrix[i][j]);
                }
            }
            printf("\n");
        }
        
        double wait_time = 1.0 - elapsed;
        if (wait_time > 0) {
            usleep((int)(wait_time * 1000000));
        }
    }
    
    matrix_cleanup();
    return 0;
}
