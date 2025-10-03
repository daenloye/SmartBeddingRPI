import ctypes
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

ROWS, COLS = 16, 12
lib = ctypes.CDLL("./libmatrix.so")
lib.init_matrix()

matrix = np.zeros((ROWS, COLS), dtype=np.uint16)
buffer = (ctypes.c_uint16 * (ROWS*COLS))()

fig, ax = plt.subplots()
im = ax.imshow(matrix, cmap='viridis', vmin=0, vmax=4095)
plt.colorbar(im)

def update(frame):
    lib.sample_matrix(buffer)
    for i in range(ROWS):
        for j in range(COLS):
            matrix[i,j] = buffer[i*COLS+j]
    im.set_array(matrix)
    return [im]

ani = FuncAnimation(fig, update, interval=100)
plt.show()
