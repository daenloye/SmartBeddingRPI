# PressureAdquisition.py
import ctypes
import numpy as np
import time
from datetime import datetime
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal,  QTimer

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

        self.interval = interval

        # Hilo
        self.thread = QThread()
        self.worker = PressureWorker(lib_path)
        self.worker.moveToThread(self.thread)
        self.thread.started.connect(self.worker.run)

        # Última muestra
        self.data = None
        self.worker.new_data.connect(self.handle_new_data)

        # Timer EXACTO
        self.timer = QTimer(self)
        self.timer.setInterval(int(self.interval * 1000))
        self.timer.timeout.connect(self.on_timeout)

    def handle_new_data(self, data):
        self.data = data.copy()

    def on_timeout(self):
        if self.data is not None:
            timestamp = datetime.now().strftime('%H:%M:%S.%f')[:-3]
            self.new_sample.emit(timestamp, self.data.copy())

    def start(self):
        self.thread.start()
        self.timer.start()

    def stop(self):
        self.worker.stop()
        self.thread.quit()
        self.thread.wait()
