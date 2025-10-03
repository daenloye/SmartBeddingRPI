import numpy as np
import matplotlib.pyplot as plt
import RPi.GPIO as GPIO
import time

# ---- Periféricos ----
import busio
import board
import digitalio
from adafruit_mcp230xx.mcp23017 import MCP23017
from adafruit_ads1x15.ads1015 import ADS1015
from adafruit_ads1x15.analog_in import AnalogIn

# ===============================
# CONFIGURACIÓN DE HARDWARE
# ===============================

# Pines del shift register
DATA_PIN = 5
SHIFT_CLOCK_PIN = 13
LATCH_CLOCK_PIN = 6

GPIO.setmode(GPIO.BCM)
GPIO.setup(DATA_PIN, GPIO.OUT)
GPIO.setup(SHIFT_CLOCK_PIN, GPIO.OUT)
GPIO.setup(LATCH_CLOCK_PIN, GPIO.OUT)

def shift_register_out(val, bits=16):
    """Saca un valor al registro de desplazamiento (MSB primero)."""
    for i in range(bits-1, -1, -1):
        bit = (int(val) >> i) & 0x1
        GPIO.output(DATA_PIN, bit)
        GPIO.output(SHIFT_CLOCK_PIN, GPIO.HIGH)
        time.sleep(0.00001)
        GPIO.output(SHIFT_CLOCK_PIN, GPIO.LOW)
    GPIO.output(LATCH_CLOCK_PIN, GPIO.HIGH)
    time.sleep(0.00001)
    GPIO.output(LATCH_CLOCK_PIN, GPIO.LOW)

# I2C
i2c = busio.I2C(board.SCL, board.SDA)

# MCP23017 en 0x21
mcp = MCP23017(i2c, address=0x21)
addr_pins = [mcp.get_pin(i) for i in range(4)]  # GPA0–GPA3
enable_pin = mcp.get_pin(4)

for pin in addr_pins:
    pin.direction = digitalio.Direction.OUTPUT
enable_pin.direction = digitalio.Direction.OUTPUT

def set_column(col_index):
    """Activa columna según índice 0–11."""
    val = int(col_index)
    for i in range(4):
        addr_pins[i].value = (val >> i) & 1
    enable_pin.value = 1  # habilita la columna

# ADS1015 en 0x48
ads = ADS1015(i2c, address=0x48)
chan = AnalogIn(ads, 0)

# ===============================
# MATRIZ DE ESCANEO
# ===============================

ROW_SIZE = 16
COL_SIZE = 12

rowArray = np.array([
    0b1000000000000000,
    0b0100000000000000,
    0b0010000000000000,
    0b0001000000000000,
    0b0000100000000000,
    0b0000010000000000,
    0b0000001000000000,
    0b0000000100000000,
    0b0000000010000000,
    0b0000000001000000,
    0b0000000000100000,
    0b0000000000010000,
    0b0000000000001000,
    0b0000000000000100,
    0b0000000000000010,
    0b0000000000000001
])

colArray = np.arange(12)  # 0–11

# ===============================
# LECTURA DE MATRIZ
# ===============================

def read_matrix():
    matrix = np.zeros((ROW_SIZE, COL_SIZE))
    for i, row_val in enumerate(rowArray):
        shift_register_out(row_val)  # activa fila
        for j, col_val in enumerate(colArray):
            set_column(col_val)
            time.sleep(0.001)  # estabilidad
            matrix[i][j] = chan.value
    return matrix

# ===============================
# LOOP PRINCIPAL CON PLOTEO
# ===============================

def main():
    plt.ion()
    fig, ax = plt.subplots()
    img = ax.imshow(np.zeros((ROW_SIZE, COL_SIZE)), cmap="viridis", vmin=0, vmax=2000)
    plt.colorbar(img, ax=ax)
    plt.title("Lectura en vivo de matriz 16x12")

    try:
        while True:
            mat = read_matrix()
            img.set_data(mat)
            plt.draw()
            plt.pause(0.01)  # refresco ~100 fps máx
    except KeyboardInterrupt:
        print("Cerrando...")
    finally:
        GPIO.cleanup()
        plt.close(fig)

if __name__ == "__main__":
    main()
