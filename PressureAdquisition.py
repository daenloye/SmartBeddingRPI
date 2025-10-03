# PressureAdquisition.py
import ctypes
import numpy as np
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal

ROW_SIZE = 16
COL_SIZE = 12


class PressureWorker(QObject):
    # Señal opcional: emite cada muestra (por si el controlador quiere recibirla)
    new_sample = pyqtSignal(np.ndarray)

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

        # Timer de muestreo (no arranca solo)
        self.timer = QTimer()
        self.timer.setInterval(1000)  # 1 segundo
        self.timer.timeout.connect(self.read)

    def start_sampling(self):
        """Comienza el muestreo periódico"""
        if not self.timer.isActive():
            self.timer.start()

    def stop_sampling(self):
        """Detiene el muestreo"""
        if self.timer.isActive():
            self.timer.stop()

    def read(self):
        """Lee la matriz desde C y la emite/imprime"""
        self.lib.matrix_update(self.buf)
        self.matrix[:, :] = np.frombuffer(self.buf, dtype=np.uint16).reshape(ROW_SIZE, COL_SIZE)

        # Imprimir en consola (debug)
        # print("Nueva muestra de presión:")
        # print(self.matrix)

        # Emitir señal (opcional)
        self.new_sample.emit(self.matrix.copy())


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
