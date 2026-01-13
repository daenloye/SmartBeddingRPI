import ctypes
import numpy as np
import matplotlib.pyplot as plt
import time

ROW_SIZE = 16
COL_SIZE = 12

# Cargar librería C
lib = ctypes.CDLL("./libmatrix.so")
lib.matrix_init()
lib.matrix_update.argtypes = [ctypes.POINTER(ctypes.c_uint16)]

# Buffer y matriz
matrix = np.zeros((ROW_SIZE, COL_SIZE), dtype=np.uint16)
buf = (ctypes.c_uint16 * (ROW_SIZE*COL_SIZE))()

# Configurar Matplotlib
plt.ion()  # modo interactivo
fig, ax = plt.subplots()
im = ax.imshow(matrix, cmap='viridis', vmin=0, vmax=2000)
plt.colorbar(im)
plt.show()

interval = 1.0  # segundos
frame = 0

try:
    while True:
        start_time = time.perf_counter()

        # Leer matriz del C
        lib.matrix_update(buf)
        matrix[:, :] = np.frombuffer(buf, dtype=np.uint16).reshape(ROW_SIZE, COL_SIZE)

        # Actualizar imagen
        im.set_array(matrix)
        ax.set_title(f"Frame {frame} | Timestamp: {time.strftime('%H:%M:%S')}")
        fig.canvas.draw()
        fig.canvas.flush_events()

        # Imprimir timestamp en consola
        print(f"Frame {frame} | Timestamp: {time.strftime('%H:%M:%S')} | Perf: {start_time:.3f}")
        print( matrix )  # Mostrar la matriz en consola

        frame += 1

        # Esperar hasta el siguiente segundo exacto
        elapsed = time.perf_counter() - start_time
        sleep_time = interval - elapsed
        if sleep_time > 0:
            time.sleep(sleep_time)

except KeyboardInterrupt:
    print("Finalizando...")
