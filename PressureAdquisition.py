# PressureAdquisition.py
import ctypes
import numpy as np
import time
from datetime import datetime
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal

ROW_SIZE = 16
COL_SIZE = 12



class PressureWorker(QObject):
    new_sample = pyqtSignal(str,np.ndarray)

    def __init__(self, lib_path="./libmatrix.so"):
        super().__init__()
        # ---------------------------
        # Inicialización de hardware
        # ---------------------------
        self.lib = ctypes.CDLL(lib_path)
        self.lib.matrix_init()
        self.lib.matrix_update.argtypes = [ctypes.POINTER(ctypes.c_uint16)]

        # Buffers
        self.matrix = np.zeros((ROW_SIZE, COL_SIZE), dtype=np.uint16)
        self.buf = (ctypes.c_uint16 * (ROW_SIZE * COL_SIZE))()

        # Variables de sincronización
        self.interval = 1.0  # segundos exactos
        self.next_time = None
        self.sampling = False

    def start_sampling(self):
        """Comienza el muestreo periódico exacto a 1 Hz"""
        if not self.sampling:
            self.sampling = True
            self.next_time = time.perf_counter() + self.interval
            self._schedule_next()

    def stop_sampling(self):
        """Detiene el muestreo"""
        self.sampling = False
        self.next_time = None

    def _schedule_next(self):
        """Programa el próximo tick exactamente"""
        if not self.sampling or self.next_time is None:
            return

        now = time.perf_counter()
        delay = max(0, self.next_time - now)  # segundos → no negativo
        QTimer.singleShot(int(delay * 1000), self._tick)

    def _tick(self):
        if not self.sampling:
            return

        self.read()

        # Avanza al próximo instante exacto
        self.next_time += self.interval

        # Si hubo retraso acumulado, corrige para no perder sincronía
        now = time.perf_counter()
        if now > self.next_time + self.interval:
            self.next_time = now + self.interval

        self._schedule_next()

    def read(self):
        # Debug con milisegundos
        timestamp=datetime.now().strftime('%H:%M:%S.%f')[:-3]
        # print(f"[{timestamp}] Nueva muestra")

        self.lib.matrix_update(self.buf)
        self.matrix[:, :] = np.frombuffer(self.buf, dtype=np.uint16).reshape(ROW_SIZE, COL_SIZE)

        self.new_sample.emit(timestamp,self.matrix.copy())

class PressureReader:
    def __init__(self, lib_path="./libmatrix.so", loop=None, logger=None):
        # Hilo de Qt
        self.thread = QThread()
        self.worker = PressureWorker(lib_path)

        # Mover el worker al hilo
        self.worker.moveToThread(self.thread)

        # Guardar referencia a worker.start_sampling para cuando se inicie el hilo
        self.thread.started.connect(lambda: print("[PressureReader] Hilo listo"))

    def start(self):
        """Inicia el hilo (pero no empieza a muestrear aún)"""
        self.thread.start()

    def begin_sampling(self):
        """Ordena al worker que comience a muestrear"""
        self.worker.start_sampling()

    def stop_sampling(self):
        """Detiene el muestreo pero mantiene el hilo vivo"""
        self.worker.stop_sampling()

    def shutdown(self):
        """Cierra todo el hilo y detiene worker"""
        self.worker.stop_sampling()
        self.thread.quit()
        self.thread.wait()
