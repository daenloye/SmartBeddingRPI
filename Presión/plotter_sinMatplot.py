import ctypes
import time

ROW_SIZE = 16
COL_SIZE = 12

# Cargar librería
lib = ctypes.CDLL("./libmatrix.so")
lib.matrix_init()
lib.matrix_update.argtypes = [ctypes.POINTER(ctypes.c_uint16)]

# Buffer
buf = (ctypes.c_uint16 * (ROW_SIZE*COL_SIZE))()

interval = 1.0  # segundos
frame = 0

try:
    while True:
        start_time = time.perf_counter()

        # Actualizar matriz (solo lectura)
        lib.matrix_update(buf)

        # Timestamp actual
        print(f"Frame {frame} | Timestamp: {time.strftime('%H:%M:%S')} | Perf: {start_time:.3f}")

        frame += 1

        # Esperar hasta el siguiente segundo exacto
        elapsed = time.perf_counter() - start_time
        sleep_time = interval - elapsed
        if sleep_time > 0:
            time.sleep(sleep_time)
except KeyboardInterrupt:
    print("Finalizando...")
