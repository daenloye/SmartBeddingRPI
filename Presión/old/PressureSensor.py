import ctypes
from PyQt6.QtCore import QThread, pyqtSignal
from typing import List
import time

ROWS = 16
COLS = 12

# ----------------- C library -----------------
LIB_NAME = "./libsensor.so"
lib = ctypes.CDLL(LIB_NAME)
lib.read_pressure_grid.argtypes = [ctypes.c_int, ctypes.POINTER(ctypes.c_double), ctypes.c_int, ctypes.c_int]
lib.read_pressure_grid.restype = None

# ----------------- PressureSensor -----------------
class PressureSensor(QThread):
    grid_ready = pyqtSignal(object)  # emitirá List[List[float]]

    def __init__(self, port: int = 1, freq_hz: float = 20.0, parent=None):
        super().__init__(parent)
        self.port = port
        self.period_s = 1.0 / freq_hz
        self._running = False       # hilo en espera
        self._sampling = False      # controla si se está muestreando

    def run(self):
        BufType = ctypes.c_double * (ROWS * COLS)
        iteration = 0
        start_time = time.time()

        while True:
            if not self._running:
                break  # salir del hilo si stop_sampling() fue llamado

            if self._sampling:
                next_time = start_time + iteration * self.period_s

                # Llamada a la librería C
                cbuf = BufType()
                lib.read_pressure_grid(self.port, cbuf, ROWS, COLS)

                # Convertir a lista de listas
                grid: List[List[float]] = [
                    [float(cbuf[r*COLS + c]) for c in range(COLS)] for r in range(ROWS)
                ]

                # Emitir la señal
                self.grid_ready.emit(grid)

                iteration += 1

                # Esperar hasta el siguiente tick
                now = time.time()
                sleep_time = next_time - now
                if sleep_time > 0:
                    time.sleep(sleep_time)
            else:
                # Si no se está muestreando, dormir un poco para no busy-wait
                time.sleep(0.01)

    # ----------------- Métodos públicos -----------------
    def start_sampling(self):
        """Inicia el muestreo."""
        if not self.isRunning():
            self._running = True
            self.start()
        self._sampling = True

    def stop_sampling(self):
        """Detiene el muestreo, pero mantiene el hilo vivo."""
        self._sampling = False

    def stop(self):
        """Detener el hilo por completo."""
        self._sampling = False
        self._running = False
        self.wait(2000)
