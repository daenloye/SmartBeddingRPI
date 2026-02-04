import spidev
import time
import numpy as np

# --- CONFIGURACIÓN ---
REG_WHO_AM_I = 0x75
EXPECTED_ID  = 0x42

def test_spi_connection(bus, device):
    """ Intenta leer el WHO_AM_I en una combinación específica de SPI """
    try:
        spi = spidev.SpiDev()
        spi.open(bus, device)
        spi.max_speed_hz = 1000000  # Bajamos a 1MHz para asegurar estabilidad
        spi.mode = 0b11
        
        # Lectura del registro 0x75
        # Enviamos 0x75 | 0x80 (bit de lectura) y un byte extra para recibir
        msg = [REG_WHO_AM_I | 0x80, 0x00]
        reply = spi.xfer2(msg)
        
        who_am_i = reply[1]
        
        print(f"Probando SPI({bus}, {device}):")
        print(f"  -> Recibido: {hex(who_am_i)}")
        
        if who_am_i == EXPECTED_ID:
            print(f"  ✅ ¡ÉXITO! Sensor encontrado en /dev/spidev{bus}.{device}")
            return spi
        else:
            print(f"  ❌ ID incorrecto (Se esperaba {hex(EXPECTED_ID)})")
            spi.close()
            return None
            
    except Exception as e:
        print(f"  ⚠️ Error abriendo SPI({bus}, {device}): {e}")
        return None

# --- ESCANEO ---
print("Iniciando escaneo de bus SPI...")
sensor_spi = None

# Probar CE0 (Pin 24)
sensor_spi = test_spi_connection(0, 0)

# Si no funciona, probar CE1 (Pin 26)
if not sensor_spi:
    sensor_spi = test_spi_connection(0, 1)

if not sensor_spi:
    print("\n--- RESUMEN ---")
    print("No se detectó el ICM42605 en ningún canal.")
    print("1. Verifica que el SPI esté activo en 'sudo raspi-config'.")
    print("2. Revisa el cableado:")
    print("   SCK  -> Pin 23")
    print("   MOSI -> Pin 19")
    print("   MISO -> Pin 21")
    print("   CS   -> Pin 24 (si usas 0,0) o Pin 26 (si usas 0,1)")
    exit()

# --- CONTINUACIÓN DEL CÓDIGO (Si lo encuentra) ---

def to_signed_16bit(val):
    return val - 0x10000 if val & 0x8000 else val

def read_registers(spi, reg_addr, length=1):
    reg_addr |= 0x80
    response = spi.xfer2([reg_addr] + [0x00] * length)
    return response[1:]

print("\nLeyendo datos de prueba (3 muestras):")
for i in range(3):
    raw_data = read_registers(sensor_spi, 0x1F, 12) # Lee Accel y Gyro
    # Procesamiento básico para ver que cambian los números
    ax = to_signed_16bit((raw_data[0] << 8) | raw_data[1])
    print(f"Muestra {i}: Accel_X_Raw = {ax}")
    time.sleep(0.5)

sensor_spi.close()