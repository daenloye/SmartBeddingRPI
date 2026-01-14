# PressureAdquisition.py
import ctypes
import numpy as np
import time
from datetime import datetime
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal,  QTimer
from ClockWorker import ClockWorker

ROW_SIZE = 16
COL_SIZE = 12

# PressureAdquisition.py
import ctypes
import numpy as np
import time
from PyQt5.QtCore import QObject, pyqtSignal

ROW_SIZE = 16
COL_SIZE = 12

class PressureWorker(QObject):
    new_data = pyqtSignal(np.ndarray)

    def __init__(self, lib_path="./libmatrix.so"):
        super().__init__()

        # Cargar librería
        self.lib = ctypes.CDLL(lib_path)
        self.lib.matrix_init()
        self.lib.matrix_update.argtypes = [ctypes.POINTER(ctypes.c_uint16)]

        # Buffers
        self.matrix = np.zeros((ROW_SIZE, COL_SIZE), dtype=np.uint16)
        self.buf = (ctypes.c_uint16 * (ROW_SIZE * COL_SIZE))()

        self.running = False

    def run(self):
        self.running = True

        while self.running:
            self.lib.matrix_update(self.buf)
            self.matrix[:, :] = np.frombuffer(
                self.buf, dtype=np.uint16
            ).reshape(ROW_SIZE, COL_SIZE)

            self.new_data.emit(self.matrix.copy())

            time.sleep(0.1)  # ~10 Hz (ajustable)

    def stop(self):
        self.running = False


class PressureReader(QObject):
    new_sample = pyqtSignal(str, np.ndarray)

    def __init__(self, lib_path="./libmatrix.so", interval=1.0):
        super().__init__()

        # -------------------------
        # Thread de adquisición
        # -------------------------
        self.sensor_thread = QThread()
        self.worker = PressureWorker(lib_path)
        self.worker.moveToThread(self.sensor_thread)
        self.sensor_thread.started.connect(self.worker.run)

        # -------------------------
        # Thread de clock exacto
        # -------------------------
        self.clock_thread = QThread()
        self.clock = ClockWorker(interval)
        self.clock.moveToThread(self.clock_thread)
        self.clock_thread.started.connect(self.clock.run)
        self.clock.tick.connect(self.on_tick)

        # -------------------------
        # Última muestra disponible
        # -------------------------
        self.data = None
        self.worker.new_data.connect(self.handle_new_data)

    def handle_new_data(self, data):
        self.data = data.copy()

    def on_tick(self):
        if self.data is not None:
            timestamp = datetime.now().strftime('%H:%M:%S.%f')[:-3]
            self.new_sample.emit(timestamp, self.data.copy())

    def start(self):
        self.sensor_thread.start()
        self.clock_thread.start()

    def stop(self):
        self.worker.stop()
        self.clock.stop()

        self.sensor_thread.quit()
        self.clock_thread.quit()

        self.sensor_thread.wait()
        self.clock_thread.wait()

