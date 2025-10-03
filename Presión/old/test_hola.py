import ctypes
import time

# Cargar la librería compartida
lib = ctypes.CDLL("./libhola.so")

# Definir prototipos
lib.start_hola.argtypes = []
lib.start_hola.restype = None

lib.stop_hola.argtypes = []
lib.stop_hola.restype = None

lib.get_data.argtypes = [ctypes.c_char_p, ctypes.c_int]
lib.get_data.restype = None

def main():
    lib.start_hola()

    start_time = time.time()
    duration = 5.0  # segundos

    buffer = ctypes.create_string_buffer(128)
    period = 1.0 / 20.0  # 20 Hz → 0.05 s

    next_time = start_time

    while True:
        # Leer dato desde C
        lib.get_data(buffer, ctypes.sizeof(buffer))
        msg = buffer.value.decode("utf-8")

        timestamp = time.time()
        print(f"{timestamp:.3f} -> {msg}")

        # Busy-wait hasta el próximo tick
        next_time += period
        while time.time() < next_time:
            pass

        if timestamp - start_time >= duration:
            break

    lib.stop_hola()

if __name__ == "__main__":
    main()
