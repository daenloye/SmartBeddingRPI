import sys
import time
import numpy as np
from PyQt6.QtCore import QCoreApplication
from PressureSensor import PressureSensor

ROWS = 16
COLS = 12

class App(QCoreApplication):
    def __init__(self, argv):
        super().__init__(argv)

        self.sensor = PressureSensor(port=1, freq_hz=20.0)
        self.sensor.grid_ready.connect(self.on_grid_ready)

        self.reads = 0
        self.max_reads = 20

        # No arrancamos automáticamente
        # self.sensor.start_sampling()  <-- ahora lo haremos manual

    def start_sensor(self):
        print("Iniciando muestreo del sensor...")
        self.sensor.start_sampling()

    def on_grid_ready(self, grid):
        self.reads += 1
        timestamp = time.time()
        print(f"\n{timestamp:.3f} [Lectura {self.reads}] Matriz {ROWS}x{COLS}:")
        print(" Primera fila:", ["{:.2f}".format(x) for x in grid[0]])
        
        mat=np.array(grid)
        print(" Media:", np.mean(mat), " Desv. típica:", np.std(mat))
        print(" Máximo:", np.max(mat), " Mínimo:", np.min(mat))
        print(" Sumatorio total:", np.sum(mat))
        print(" Formato interno:", mat.shape, mat.dtype)

        if self.reads >= self.max_reads:
            print("Máximo de lecturas alcanzado. Deteniendo...")
            self.shutdown()

    def shutdown(self):
        self.sensor.stop()
        self.quit()


if __name__ == "__main__":
    app = App(sys.argv)

    # Ejemplo: iniciar muestreo después de 2 segundos
    from PyQt6.QtCore import QTimer
    QTimer.singleShot(2000, app.start_sensor)

    sys.exit(app.exec())
