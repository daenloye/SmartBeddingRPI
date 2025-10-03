import smbus2
import RPi.GPIO as GPIO
import time
import numpy as np

# --------------------- Configuración ---------------------
# Pines shift register (BCM)
DATA_PIN = 5
SHIFT_CLOCK_PIN = 13
LATCH_CLOCK_PIN = 6

GPIO.cleanup()
GPIO.setmode(GPIO.BCM)
GPIO.setup(DATA_PIN, GPIO.OUT)
GPIO.setup(SHIFT_CLOCK_PIN, GPIO.OUT)
GPIO.setup(LATCH_CLOCK_PIN, GPIO.OUT)

# I2C
bus = smbus2.SMBus(1)
MCP_ADDR = 0x21
ADS_ADDR = 0x48

# Matriz
ROWS = 16
COLS = 12
matrix = np.zeros((ROWS, COLS), dtype=np.uint16)

# Filas y columnas
rowArray = [
    0b1000000000000000,0b0100000000000000,0b0010000000000000,0b0001000000000000,
    0b0000100000000000,0b0000010000000000,0b0000001000000000,0b0000000100000000,
    0b0000000010000000,0b0000000001000000,0b0000000000100000,0b0000000000010000,
    0b0000000000001000,0b0000000000000100,0b0000000000000010,0b0000000000000001
]
colArray = list(range(COLS))  # columnas 0..11

# --------------------- Funciones ---------------------

def shift_register_out(val, bits=16):
    for i in range(bits-1, -1, -1):
        bit = (val >> i) & 0x1
        GPIO.output(DATA_PIN, bit)
        GPIO.output(SHIFT_CLOCK_PIN, GPIO.HIGH)
        time.sleep(0.00001)
        GPIO.output(SHIFT_CLOCK_PIN, GPIO.LOW)
    GPIO.output(LATCH_CLOCK_PIN, GPIO.HIGH)
    time.sleep(0.00001)
    GPIO.output(LATCH_CLOCK_PIN, GPIO.LOW)

def set_column(col):
    # MCP23017 GPIOA=0x12, GPIOB=0x13
    # Activamos la columna en GPIOA, enable en GPIOB0
    bus.write_byte_data(MCP_ADDR, 0x12, 1<<col)
    bus.write_byte_data(MCP_ADDR, 0x13, 1)  # enable pin

def read_ads1015(channel=0):
    if channel>3: channel=0
    config = 0x8000 | (channel<<12) | 0x0200 | 0x0003  # OS=1, 1600SPS
    buf = [(config>>8)&0xFF, config&0xFF]
    bus.write_i2c_block_data(ADS_ADDR, 0x01, buf)

    # Esperar que el bit OS=1 indique conversión completa
    for _ in range(100):
        raw_config = bus.read_i2c_block_data(ADS_ADDR, 0x01, 2)
        val = (raw_config[0]<<8) | raw_config[1]
        if val & 0x8000:
            break
        time.sleep(0.001)  # 1ms

    # Leer registro de conversión
    raw = bus.read_i2c_block_data(ADS_ADDR, 0x00, 2)
    value = (raw[0]<<8 | raw[1]) >> 4
    return value

# --------------------- Muestreo ---------------------

for i, row in enumerate(rowArray):
    shift_register_out(row)
    for j, col in enumerate(colArray):
        set_column(col)
        time.sleep(0.001)
        matrix[i,j] = read_ads1015(j%4)  # ADS1015 tiene 4 canales

print(matrix)
GPIO.cleanup()