import board
import neopixel
import time

# Configuración
LED_PIN = board.D16   # GPIO16
NUM_LEDS = 1           # cantidad de LEDs en la tira
BRIGHTNESS = 0.2       # brillo (0.0 a 1.0)

# Inicializa el LED
pixels = neopixel.NeoPixel(LED_PIN, NUM_LEDS, brightness=BRIGHTNESS, auto_write=False)

# Secuencia básica de colores
try:
    while True:
        pixels.fill((255, 0, 0))   # rojo
        pixels.show()
        time.sleep(1)

        pixels.fill((0, 255, 0))   # verde
        pixels.show()
        time.sleep(1)

        pixels.fill((0, 0, 255))   # azul
        pixels.show()
        time.sleep(1)

except KeyboardInterrupt:
    pixels.fill((0, 0, 0))
    pixels.show()
