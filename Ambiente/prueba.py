import smbus2
import time 
bus = smbus2.SMBus(1)
address_temp = 0x44
# Comando para medición de alta precisión sin reloj stretching
bus.write_i2c_block_data(address_temp, 0x24, [0x00])
time.sleep(0.5)

data = bus.read_i2c_block_data(address_temp, 0x00, 6)

raw_temp = data[0] << 8 | data[1]
raw_hum = data[3] << 8 | data[4]

temp = -45 + (175 * raw_temp / 65535.0)
humidity = 100 * raw_hum / 65535.0

print(f"Temperatura: {temp:.2f} °C")
print(f"Humedad: {humidity:.2f} %")