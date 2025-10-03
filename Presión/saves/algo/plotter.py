import ctypes
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

ROW_SIZE = 16
COL_SIZE = 12

lib = ctypes.CDLL("./libmatrix.so")
lib.matrix_init()
lib.matrix_update.argtypes = [ctypes.POINTER(ctypes.c_uint16)]

matrix = np.zeros((ROW_SIZE,COL_SIZE),dtype=np.uint16)

fig, ax = plt.subplots()
im = ax.imshow(matrix, cmap='viridis', vmin=0, vmax=4095)
plt.colorbar(im)

def update(frame):
    buf = (ctypes.c_uint16 * (ROW_SIZE*COL_SIZE))()
    lib.matrix_update(buf)
    matrix[:,:] = np.frombuffer(buf,dtype=np.uint16).reshape(ROW_SIZE,COL_SIZE)
    im.set_array(matrix)
    return [im]

ani = FuncAnimation(fig, update, interval=50)
plt.show()
