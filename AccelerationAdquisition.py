import time
import numpy as np
import spidev
from datetime import datetime
from PyQt5.QtCore import QObject, QThread, pyqtSignal

# -----------------------------
# Constantes del ICM42605
# -----------------------------
REG_WHO_AM_I           = 0x75
ICM42605_PWR_MGMT0     = 0x4E
ICM42605_GYRO_CONFIG0  = 0x4F
ICM42605_ACCEL_CONFIG0 = 0x50
TEMP_REG               = 0x1D

# Configuración de escalas
GFS_250DPS  = 0x03
AFS_2G      = 0x03
GODR_1000Hz = 0x06
AODR_1000Hz = 0x06

# Sensibilidad
aRes = (2**(AFS_2G+1)) / 32768   # g/LSB
gRes = (2000/(2**(GFS_250DPS))) / 32768  # dps/LSB


# -----------------------------
# Worker (hilo real de adquisición)
# -----------------------------
class AccelerationWorker(QObject):
    new_sample = pyqtSignal(str,np.ndarray)

    def __init__(self, bus=0, device=0, interval=0.02):
        """
        Worker que lee datos del ICM42605 vía SPI
        :param bus: Bus SPI
        :param device: Device SPI
        :param interval: intervalo de muestreo en segundos (default 0.02s = 50Hz)
        """
        super().__init__()
        self.spi = spidev.SpiDev()
        self.bus = bus
        self.device = device
        self.interval = interval
        self.running = False

    # --- Inicialización de hardware ---
    def init_sensor(self):
        self.spi.open(self.bus, self.device)
        self.spi.max_speed_hz = 1_000_000
        self.spi.mode = 0b00

        # Configurar sensor
        self.write_reg(ICM42605_PWR_MGMT0, 0x0F)   # habilita acelerómetro y giroscopio
        self.write_reg(ICM42605_GYRO_CONFIG0, (GFS_250DPS << 5) | GODR_1000Hz)
        self.write_reg(ICM42605_ACCEL_CONFIG0, (AFS_2G << 5) | AODR_1000Hz)

    def write_reg(self, reg, val):
        reg_addr = reg & 0x7F
        self.spi.xfer2([reg_addr, val])

    def read_regs(self, reg, length=1):
        reg_addr = reg | 0x80
        response = self.spi.xfer2([reg_addr] + [0x00] * length)
        return response[1:]

    def to_signed(self, val):
        return val - 0x10000 if val & 0x8000 else val

    # --- Bucle principal de muestreo ---
    def run(self):
        self.init_sensor()
        self.running = True
        next_time = time.perf_counter() + self.interval

        while self.running:
            now = time.perf_counter()
            if now >= next_time:

                timestamp=datetime.now().strftime('%H:%M:%S.%f')[:-3]
                # print(f"[{timestamp}] Nueva muestra acel")

                gx, gy, gz, ax, ay, az = self.read_sensor()
                data = np.array([gx, gy, gz, ax, ay, az], dtype=float)
                self.new_sample.emit(timestamp,data)

                # siguiente instante exacto
                next_time += self.interval
                if now > next_time + self.interval:
                    next_time = now + self.interval
            else:
                time.sleep(0.005)  # evita 100% CPU

        self.spi.close()

    def read_sensor(self):
        raw = self.read_regs(TEMP_REG, length=14)

        gx = self.to_signed((raw[0] << 8) | raw[1]) * gRes
        gy = self.to_signed((raw[2] << 8) | raw[3]) * gRes
        gz = self.to_signed((raw[4] << 8) | raw[5]) * gRes
        ax = self.to_signed((raw[6] << 8) | raw[7]) * aRes
        ay = self.to_signed((raw[8] << 8) | raw[9]) * aRes
        az = self.to_signed((raw[10] << 8) | raw[11]) * aRes

        return gx, gy, gz, ax, ay, az

    def stop(self):
        self.running = False


# -----------------------------
# Reader (fachada que maneja hilo y worker)
# -----------------------------
class AccelerationReader(QObject):
    new_sample = pyqtSignal(str,np.ndarray)

    def __init__(self, bus=0, device=0, interval=0.02):
        super().__init__()
        self.thread = QThread()
        self.worker = AccelerationWorker(bus, device, interval)
        self.worker.moveToThread(self.thread)

        self.worker.new_sample.connect(self.new_sample)
        self.thread.started.connect(self.worker.run)

    def start(self):
        self.thread.start()

    def stop(self):
        self.worker.stop()
        self.thread.quit()
        self.thread.wait()
