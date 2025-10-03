import os
import sys
import ctypes
import numpy as np
import pyqtgraph as pg
from pyqtgraph.Qt import QtWidgets, QtCore
from PyQt5.QtCore import QThread, pyqtSignal
from scipy.ndimage import zoom

ROWS = 16
COLS = 12

# -----------------------------
# Cargar librería .so
# -----------------------------
LIB_NAME = os.path.join(os.path.dirname(__file__), "libsensor.so")
lib = ctypes.CDLL(LIB_NAME)
lib.read_pressure_grid.argtypes = [
    ctypes.c_int,
    ctypes.POINTER(ctypes.c_double),
    ctypes.c_int,
    ctypes.c_int
]
lib.read_pressure_grid.restype = None

# -----------------------------
# Thread para muestreo
# -----------------------------
class SensorThread(QThread):
    new_frame = pyqtSignal(np.ndarray)

    def run(self):
        buffer = (ctypes.c_double * (ROWS * COLS))()
        while True:
            lib.read_pressure_grid(0, buffer, ROWS, COLS)
            mat = np.ctypeslib.as_array(buffer).reshape((ROWS, COLS))
            self.new_frame.emit(mat)

# -----------------------------
# Configuración PyQtGraph
# -----------------------------
app = QtWidgets.QApplication(sys.argv)
win = pg.GraphicsLayoutWidget(show=True)
win.setWindowTitle("Mapa de presión 16x12")

plot = win.addPlot()
img_item = pg.ImageItem()
plot.addItem(img_item)

plot.setLabel('left', 'Filas')
plot.setLabel('bottom', 'Columnas')
plot.setLimits(xMin=0, xMax=COLS, yMin=0, yMax=ROWS)
plot.setAspectLocked(False)

# Escala de colores
img_item.setLevels([0, 2000])  # Ajusta según rango de ADS1015
cmap = pg.colormap.get("viridis")
img_item.setLookupTable(cmap.getLookupTable())

# -----------------------------
# Función para actualizar GUI
# -----------------------------
def handle_frame(mat):
    # Interpolamos para que se vea más grande
    mat_zoomed = zoom(mat, (10,10), order=1)  # factor 10
    img_item.setImage(mat_zoomed, autoLevels=False)

# -----------------------------
# Iniciar thread
# -----------------------------
sensor_thread = SensorThread()
sensor_thread.new_frame.connect(handle_frame)
sensor_thread.start()

# -----------------------------
# Ejecutar aplicación
# -----------------------------
if __name__ == "__main__":
    sys.exit(app.exec_())
