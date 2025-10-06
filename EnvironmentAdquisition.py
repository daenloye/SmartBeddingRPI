import time
from datetime import datetime
from PyQt5.QtCore import QObject, QTimer, pyqtSignal
import smbus2


class EnvironmentManager(QObject):
    # Señal que emite temperatura y humedad cada 20 s
    new_sample = pyqtSignal(str, float, float)

    def __init__(self, interval=20_000, max_samples=3, logger=None, parent=None):
        """
        interval: tiempo entre muestras en milisegundos (default 20 s)
        max_samples: número máximo de muestras en el buffer
        logger: instancia de tu logger personalizado (opcional)
        """
        super().__init__(parent)
        self.interval = interval
        self.max_samples = max_samples
        self.samples = []  # [(timestamp, temp, hum)]
        self.logger = logger

        # Inicializa I2C
        self.bus = smbus2.SMBus(1)
        self.address = 0x44  # dirección del sensor SHT3x

        # Timer de muestreo
        self.timer = QTimer(self)
        self.timer.timeout.connect(self._take_sample)

    def start(self):
        """Inicia el muestreo periódico."""
        self._take_sample()  # primera muestra inmediata
        self.timer.start(self.interval)

        if self.logger:
            self.logger.log(app="EnvironmentManager", func="start", level=0,
                            msg=f"Iniciado: muestreo cada {self.interval/1000:.0f}s")

    def stop(self):
        """Detiene el muestreo."""
        self.timer.stop()
        if self.logger:
            self.logger.log(app="EnvironmentManager", func="stop", level=0,
                            msg="Detenido")

    def _take_sample(self):
        """Toma una nueva muestra de temperatura y humedad."""
        try:
            temperature, humidity = self._read_temp_humidity()
            timestamp=datetime.now().strftime('%H:%M:%S.%f')[:-3]

            # Guarda la muestra
            self.samples.append((timestamp, temperature, humidity))
            if len(self.samples) > self.max_samples:
                self.samples.pop(0)

            # Log y emisión
            if self.logger:
                self.logger.log(app="EnvironmentManager", func="_take_sample", level=1,
                                msg=f"Muestra: {temperature:.2f}°C / {humidity:.2f}%")

            # 🔥 Emitir señal con la nueva muestra
            self.new_sample.emit(timestamp, temperature, humidity)

        except Exception as e:
            if self.logger:
                self.logger.log(app="EnvironmentManager", func="_take_sample", level=2,
                                msg=f"Error leyendo sensor: {e}")

    def _read_temp_humidity(self):
        """Lee la temperatura y humedad reales del sensor SHT3x."""
        # Comando: medición de alta precisión sin clock stretching
        self.bus.write_i2c_block_data(self.address, 0x24, [0x00])
        time.sleep(0.5)

        data = self.bus.read_i2c_block_data(self.address, 0x00, 6)

        raw_temp = data[0] << 8 | data[1]
        raw_hum = data[3] << 8 | data[4]

        temp = -45 + (175 * raw_temp / 65535.0)
        humidity = 100 * raw_hum / 65535.0

        return temp, humidity

    def get_temp(self):

        # Debug con milisegundos
        timestamp=datetime.now().strftime('%H:%M:%S.%f')[:-3]

        """Devuelve el promedio actual (temperatura, humedad)."""
        if not self.samples:
            return 0.0, 0.0

        temps = [t for _, t, _ in self.samples]
        hums = [h for _, _, h in self.samples]
        avg_temp = sum(temps) / len(temps)
        avg_hum = sum(hums) / len(hums)
        return timestamp,avg_temp, avg_hum
