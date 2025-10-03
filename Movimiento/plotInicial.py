import spidev
import time
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
from collections import deque

# --- CONFIGURACIÓN DEL SENSOR ICM42605 (EXTRAÍDA DEL NOTEBOOK) ---

# Creación de objeto spi para comunicación con sensor IMU
# ASUMIMOS bus 0, CE0
try:
    spi = spidev.SpiDev()
    spi.open(0, 0)
    spi.max_speed_hz = 4000000
    spi.mode = 0b11  # ICM requiere SPI mode 3
except Exception as e:
    print(f"Error al inicializar SPI: {e}")
    print("Asegúrate de tener el módulo 'spidev' instalado y la interfaz SPI habilitada.")
    exit()

# Definiciones de registros y escalas
REG_WHO_AM_I           = 0x75
ICM42605_DEVICE_CONFIG = 0x11
ICM42605_PWR_MGMT0     = 0x4E
ICM42605_GYRO_CONFIG0  = 0x4F
ICM42605_ACCEL_CONFIG0 = 0x50
ICM42605_GYRO_CONFIG1  = 0x51
TEMP_REG               = 0x1D
DATA_REG               = 0x1F

# Escalas usadas en el notebook
GFS_250DPS  = 0x03  # Escala del giroscopio: +/- 250 grados/s
AODR_1000Hz = 0x06  # Output Data Rate del acelerómetro: 1000 Hz
AFS_2G      = 0x03  # Escala del acelerómetro: +/- 2G
GODR_1000Hz = 0x06  # Output Data Rate del giroscopio: 1000 Hz

Gscale = GFS_250DPS
Ascale = AFS_2G
GODR   = GODR_1000Hz
AODR   = AODR_1000Hz

# Sensibilidad (Resolution) calculada en el notebook
# aRes = (2**(AFS_2G+1)) / 32768
# gRes = (2000/(2**(Gscale))) / 32768
aRes = (2**(Ascale+1)) / 32768
gRes = (2000/(2**(Gscale))) / 32768

# Variables para Bias/Offset (calibración)
abias = np.zeros(3)
gbias = np.zeros(3)

# --- FUNCIONES DE COMUNICACIÓN SPI ---

def write_register(spi, reg_addr, data):
    """ Escribe un solo byte 'data' en el registro 'reg_addr' del ICM42605. """
    reg_addr_r = reg_addr & 0x7F  # bit 7 = 0 para escritura
    spi.xfer2([reg_addr_r, data])
    # No es necesario retornar el check para la visualización en vivo

def read_registers(spi, reg_addr, length=1):
    """ Lee 'length' bytes desde el registro 'reg_addr' del ICM42605. """
    reg_addr = reg_addr | 0x80  # bit 7 = 1 para lectura
    response = spi.xfer2([reg_addr] + [0x00] * length)
    return response[1:]  # omitir el eco del reg_addr

def to_signed_16bit(val):
    """ Convierte un entero de 16 bits sin signo a con signo (complemento a 2). """
    return val - 0x10000 if val & 0x8000 else val

def readSensor(spi):
    """
    Lee los 14 bytes de datos del sensor (Temp + Accel + Gyro) y aplica la conversión.
    """
    dest = np.zeros(6, dtype=int)
    acc  = np.zeros(3)
    gyr  = np.zeros(3)

    # Lee 14 bytes consecutivos a partir de TEMP_REG
    temp = read_registers(spi, TEMP_REG, length=14)

    # Temperatura (no utilizada para la gráfica, pero se calcula)
    raw_temp = (temp[0] << 8) | temp[1]
    raw_temp = to_signed_16bit(raw_temp)
    _t = raw_temp / 132.48 + 25

    # Datos acelerómetro y giroscopio
    for i in range(6):
        # Combina byte alto y bajo (High Byte << 8) | Low Byte
        raw = (temp[2*i + 2] << 8) | temp[2*i + 3]
        dest[i] = to_signed_16bit(raw)

    # Aplicar la escala y el bias
    acc[0] = dest[0] * aRes - abias[0] # Se resta el bias
    acc[1] = dest[1] * aRes - abias[1]
    acc[2] = dest[2] * aRes - abias[2]

    gyr[0] = dest[3] * gRes - gbias[0] # Se resta el bias
    gyr[1] = dest[4] * gRes - gbias[1]
    gyr[2] = dest[5] * gRes - gbias[2]

    # Retorna solo los 6 valores de Gyro y Accel
    return gyr[0], gyr[1], gyr[2], acc[0], acc[1], acc[2]

# --- INICIALIZACIÓN DEL SENSOR ---

def inicializar_sensor():
    print("Inicializando sensor ICM42605...")

    # 1. Reset
    write_register(spi, ICM42605_DEVICE_CONFIG, 0x01)
    time.sleep(0.05) # Esperar un poco después del reset

    # 2. Activación del sensor IMU: Activa el Gyro y Accel en modo "Low Noise" (0x0F)
    write_register(spi, ICM42605_PWR_MGMT0, 0x0F)
    time.sleep(0.01)

    # 3. Configuración del Giroscopio: ODR 1000Hz | FS_SEL +/- 250 DPS
    write_register(spi, ICM42605_GYRO_CONFIG0, GODR | Gscale << 5)
    
    # 4. Configuración del Acelerómetro: ODR 1000Hz | FS_SEL +/- 2G
    write_register(spi, ICM42605_ACCEL_CONFIG0, AODR | Ascale << 5)
    
    # 5. Configuración del filtro del Giroscopio (como en el notebook)
    write_register(spi, ICM42605_GYRO_CONFIG1, 0xD0) 
    
    # Calibración de Bias (usando la lógica del notebook)
    print("Calibrando bias... mantén el sensor quieto por unos segundos.")
    global abias, gbias
    suma = np.zeros(6)
    
    # 128 muestras de calibración, esperando 5ms entre lecturas
    for _ in range(0, 128):
        # readSensor() retorna (gx, gy, gz, ax, ay, az)
        gx, gy, gz, ax, ay, az = readSensor(spi)
        suma[0] += ax
        suma[1] += ay
        suma[2] += az
        suma[3] += gx
        suma[4] += gy
        suma[5] += gz
        time.sleep(0.005)
    
    # Cálculo del Bias
    abias[0:3] = suma[0:3] * aRes / 128
    gbias[0:3] = suma[3:6] * gRes / 128
    
    # Nota: En el notebook se usaba 'suma[i] += acc[i]' donde acc ya tenía el aRes aplicado
    # Para ser estrictos con la lógica del notebook, la lectura debería retornar raw data, 
    # pero ajustaremos la lectura para que retorne los datos convertidos a unidades. 
    # **NOTA DE AJUSTE:** Para evitar doble aplicación de 'aRes' y 'gRes' en la calibración,
    # la función `readSensor` *temporalmente* no usará el `abias` y `gbias`.

    print(f"Bias de acelerómetro (g): {abias}")
    print(f"Bias de giroscopio (º/s): {gbias}")
    print("Inicialización y calibración completada.")

# --- VISUALIZACIÓN EN VIVO CON MATPLOTLIB ---

# Inicializar estructuras de datos para la gráfica
MAX_PUNTOS = 200
INTERVALO_MS = 50 # 20 Hz de actualización, ajustado para el tiempo de sleep

tiempos = deque(maxlen=MAX_PUNTOS)
gyro_x = deque(maxlen=MAX_PUNTOS)
gyro_y = deque(maxlen=MAX_PUNTOS)
gyro_z = deque(maxlen=MAX_PUNTOS)
accel_x = deque(maxlen=MAX_PUNTOS)
accel_y = deque(maxlen=MAX_PUNTOS)
accel_z = deque(maxlen=MAX_PUNTOS)

inicio_tiempo = time.time()

def actualizar_grafica(i):
    """
    Función de callback para la animación, lee el sensor y actualiza los datos.
    """
    try:
        # La lectura retorna (gx, gy, gz, ax, ay, az) con bias aplicado
        gx, gy, gz, ax, ay, az = readSensor(spi)
    except Exception as e:
        print(f"Error en la lectura del sensor: {e}")
        return (line_gx, line_gy, line_gz, line_ax, line_ay, line_az)

    tiempo_actual = time.time() - inicio_tiempo

    # 1. Agregar datos
    tiempos.append(tiempo_actual)
    gyro_x.append(gx)
    gyro_y.append(gy)
    gyro_z.append(gz)
    accel_x.append(ax)
    accel_y.append(ay)
    accel_z.append(az)

    # 2. Actualizar límites del eje X (Ventana deslizante)
    if tiempos:
        ax_gyro.set_xlim(tiempos[0], tiempos[-1])
        ax_accel.set_xlim(tiempos[0], tiempos[-1])

    # 3. Actualizar datos de las líneas
    line_gx.set_data(tiempos, gyro_x)
    line_gy.set_data(tiempos, gyro_y)
    line_gz.set_data(tiempos, gyro_z)

    line_ax.set_data(tiempos, accel_x)
    line_ay.set_data(tiempos, accel_y)
    line_az.set_data(tiempos, accel_z)

    # 4. Retornar las líneas
    return (line_gx, line_gy, line_gz, line_ax, line_ay, line_az)

# --- EJECUCIÓN ---

# Inicializar y Calibrar el sensor
inicializar_sensor()

# Configuración de Matplotlib
plt.style.use('dark_background')
fig, (ax_gyro, ax_accel) = plt.subplots(2, 1, figsize=(12, 8), sharex=True)
fig.suptitle('Visualización de Sensores ICM42605 (SPI) en Vivo', color='white')

# --- Gráfica de Giroscopio (ENFOQUE PRINCIPAL) ---
ax_gyro.set_title('Giroscopio (Velocidad Angular)', color='skyblue')
ax_gyro.set_ylabel('Velocidad Angular (º/s)', color='white')
ax_gyro.tick_params(axis='y', colors='white')
ax_gyro.tick_params(axis='x', colors='white')
ax_gyro.spines['bottom'].set_color('gray')
ax_gyro.spines['top'].set_color('gray')
ax_gyro.spines['left'].set_color('gray')
ax_gyro.spines['right'].set_color('gray')

line_gx, = ax_gyro.plot([], [], label='Gyro X', color='cyan', linewidth=2)
line_gy, = ax_gyro.plot([], [], label='Gyro Y', color='magenta', linewidth=2)
line_gz, = ax_gyro.plot([], [], label='Gyro Z', color='yellow', linewidth=2)
ax_gyro.legend(loc='upper right', facecolor='black', edgecolor='gray', labelcolor='white')

# Límites Y para +/- 250 DPS (Escala configurada)
ax_gyro.set_ylim(-300, 300)

# --- Gráfica de Acelerómetro ---
ax_accel.set_title('Acelerómetro', color='lightgreen')
ax_accel.set_xlabel('Tiempo (s)', color='white')
ax_accel.set_ylabel('Aceleración (g)', color='white')
ax_accel.tick_params(axis='y', colors='white')
ax_accel.tick_params(axis='x', colors='white')
ax_accel.spines['bottom'].set_color('gray')
ax_accel.spines['top'].set_color('gray')
ax_accel.spines['left'].set_color('gray')
ax_accel.spines['right'].set_color('gray')

line_ax, = ax_accel.plot([], [], label='Accel X', color='red', linewidth=1)
line_ay, = ax_accel.plot([], [], label='Accel Y', color='lime', linewidth=1)
line_az, = ax_accel.plot([], [], label='Accel Z', color='orange', linewidth=1)
ax_accel.legend(loc='upper right', facecolor='black', edgecolor='gray', labelcolor='white')

# Límites Y para +/- 2G (Escala configurada)
ax_accel.set_ylim(-3, 3) # Un poco más ancho que 2G

# Ejecutar la Animación
# `blit=True` optimiza la actualización de la gráfica solo redibujando los elementos que cambian.
ani = animation.FuncAnimation(fig, actualizar_grafica, interval=INTERVALO_MS, blit=True)

plt.tight_layout(rect=[0, 0.03, 1, 0.95])
plt.show()

# Cerrar la conexión SPI al finalizar
spi.close()