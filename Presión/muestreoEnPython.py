import numpy as np
import matplotlib.pyplot as plt
import RPi.GPIO as GPIO
import time
from matplotlib.animation import FuncAnimation

from adafruit_mcp230xx.mcp23017 import MCP23017
import busio
import board
import digitalio
from adafruit_ads1x15.ads1015 import ADS1015
from adafruit_ads1x15.analog_in import AnalogIn

# Pines shift register
DATA_PIN = 5
SHIFT_CLOCK_PIN = 13
LATCH_CLOCK_PIN = 6

GPIO.setmode(GPIO.BCM)
GPIO.setup(DATA_PIN, GPIO.OUT)
GPIO.setup(SHIFT_CLOCK_PIN, GPIO.OUT)
GPIO.setup(LATCH_CLOCK_PIN, GPIO.OUT)

def shift_register_out(val, bits=16):
    for i in range(bits-1, -1, -1):
        bit = (int(val) >> i) & 0x1
        GPIO.output(DATA_PIN, bit)
        GPIO.output(SHIFT_CLOCK_PIN, GPIO.HIGH)
        time.sleep(0.00001)
        GPIO.output(SHIFT_CLOCK_PIN, GPIO.LOW)
    GPIO.output(LATCH_CLOCK_PIN, GPIO.HIGH)
    time.sleep(0.00001)
    GPIO.output(LATCH_CLOCK_PIN, GPIO.LOW)

# Inicialización MCP23017
i2c = busio.I2C(board.SCL, board.SDA)
mcp = MCP23017(i2c, address=0x21)

addr_pins = [mcp.get_pin(i) for i in range(4)]
enable_pin = mcp.get_pin(4)

for pin in addr_pins:
    pin.direction = digitalio.Direction.OUTPUT
enable_pin.direction = digitalio.Direction.OUTPUT

def set_column(col_index):
    val = int(col_index)
    for i in range(4):
        addr_pins[i].value = (val >> i) & 1
    enable_pin.value = 1

# Inicialización ADS1015
ads = ADS1015(i2c, address=0x48)
chan = AnalogIn(ads, 0)

ROW_SIZE = 16
COL_SIZE = 12
matrix = np.zeros((ROW_SIZE, COL_SIZE))

rowArray = np.array([
    0b1000000000000000, 0b0100000000000000, 0b0010000000000000, 0b0001000000000000,
    0b0000100000000000, 0b0000010000000000, 0b0000001000000000, 0b0000000100000000,
    0b0000000010000000, 0b0000000001000000, 0b0000000000100000, 0b0000000000010000,
    0b0000000000001000, 0b0000000000000100, 0b0000000000000010, 0b0000000000000001
])

colArray = np.array([
    0b00010000, 0b00010001, 0b00010010, 0b00010011,
    0b00010100, 0b00010101, 0b00010110, 0b00010111,
    0b00011000, 0b00011001, 0b00011010, 0b00011011
])

# Configuración matplotlib
fig, ax = plt.subplots()
im = ax.imshow(matrix, cmap='viridis', vmin=0, vmax=2000)
plt.colorbar(im)

frame = 0

def update(frame):
    global matrix
    for i, row_idx in enumerate(rowArray):
        shift_register_out(row_idx)
        GPIO.output(LATCH_CLOCK_PIN, GPIO.HIGH)
        GPIO.output(LATCH_CLOCK_PIN, GPIO.LOW)
        for j, col_idx in enumerate(colArray):
            set_column(col_idx)
            time.sleep(0.001)  # Pequeña espera
            matrix[i, j] = chan.value
    im.set_array(matrix)

    print(f"Frame {frame}")
    print( matrix )  # Mostrar la matriz en consola

    frame += 1


    return [im]

ani = FuncAnimation(fig, update, interval=100)
plt.show()
