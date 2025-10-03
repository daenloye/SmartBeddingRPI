#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdint.h>
#include <time.h>
#include <gpiod.h>
#include <linux/i2c-dev.h>
#include <sys/ioctl.h>
#include <fcntl.h>

#define MAX_ROWS 16
#define MAX_COLS 12

// ============================
// CONFIG PINS (shift register)
// ============================
#define DATA_PIN  5
#define SHCP_PIN 13
#define STCP_PIN  6

// Variables globales para I2C y GPIO (Inicialización real omitida por el entorno)
static int fd_mcp = -1;
static int fd_ads = -1;
static struct gpiod_chip *chip = NULL;
static struct gpiod_line *data_line = NULL;
static struct gpiod_line *shcp_line = NULL;
static struct gpiod_line *stcp_line = NULL;

// [Código de shift_register_out, set_column, y read_ads1015 se mantienen igual]
// ... (omito el código de las funciones auxiliares por brevedad, asume que están aquí)

static void shift_register_out(struct gpiod_line *data, struct gpiod_line *shcp, struct gpiod_line *stcp, uint16_t val) { /* ... */ }
void set_column(int fd_i2c, int col) { /* ... */ }
int read_ads1015(int fd_ads) { /* ... */ return 0; } // Simulación de retorno

// ============================
// FUNCIÓN PRINCIPAL DE LA LIBRERÍA
// ============================

// El nombre `read_pressure_grid` coincide con el esperado por Python.
// Debe retornar void, y recibir el puntero y las dimensiones.
void read_pressure_grid(int port, double *buffer, int rows, int cols) {

    // --- 1. Inicialización (Se debe hacer una sola vez en un init si es posible) ---
    // Por simplicidad, intentamos inicializar aquí, aunque no es lo ideal.
    if (chip == NULL) {
        // Inicializar GPIO, MCP23017, ADS1015... (código del main original)
        // Omitido por simulación, pero este es el lugar donde iría.
        // Asumiendo que las líneas gpiod_chip_open_by_name, open("/dev/i2c-1"), ioctl son exitosas
        // ...
        // Establecer fd_mcp, fd_ads, data_line, shcp_line, stcp_line
    }
    // Si la inicialización falló, salimos
    if (fd_ads == -1 || fd_mcp == -1) {
        // Opcional: Llenar el buffer con ceros o un valor de error
        for(int i = 0; i < rows * cols; i++) buffer[i] = 0.0;
        return; 
    }
    // ----------------------------------------------------------------------------


    // 2. Escaneo y llenado del buffer
    // Aseguramos que las dimensiones no excedan los límites definidos en el hardware
    int current_rows = (rows > MAX_ROWS) ? MAX_ROWS : rows;
    int current_cols = (cols > MAX_COLS) ? MAX_COLS : cols;

    for (int r = 0; r < current_rows; r++) {
        // Activar la fila (usa las líneas GPIO inicializadas)
        uint16_t row_val = (1 << (15 - r));
        shift_register_out(data_line, shcp_line, stcp_line, row_val);

        for (int c = 0; c < current_cols; c++) {
            // Activar la columna (usa el descriptor de archivo del MCP23017)
            set_column(fd_mcp, c);
            usleep(1000); // estabilidad

            // Leer el valor del sensor (usa el descriptor de archivo del ADS1015)
            int val_ads = read_ads1015(fd_ads);

            // 3. ¡LLENAR EL BUFFER EN EL ORDEN CORRECTO!
            // El índice debe coincidir con la convención de Python: Fila-Principal
            int buffer_index = r * current_cols + c;
            
            // Convertimos el valor int del ADS1015 a double para el buffer
            buffer[buffer_index] = (double)val_ads;
        }
    }
    
    // Opcional: Espera para cumplir con el tiempo total de muestreo si es necesario, 
    // pero el Python Thread ya maneja la temporización externa.
}