import ctypes
import numpy as np

ROW_SIZE = 16
COL_SIZE = 12

class PressureReader:
    def __init__(self, lib_path="./libmatrix.so", logger=None, loop=None):
        # Cargar librería C
        self.lib = ctypes.CDLL(lib_path)
        self.lib.matrix_init()
        self.lib.matrix_update.argtypes = [ctypes.POINTER(ctypes.c_uint16)]

        # Buffer y matriz
        self.matrix = np.zeros((ROW_SIZE, COL_SIZE), dtype=np.uint16)
        self.buf = (ctypes.c_uint16 * (ROW_SIZE * COL_SIZE))()

        #Logger y loop
        self.logger = logger
        self.loop = loop

    def read(self):
        self.lib.matrix_update(self.buf)
        self.matrix[:, :] = np.frombuffer(self.buf, dtype=np.uint16).reshape(ROW_SIZE, COL_SIZE)
        return self.matrix.copy()
