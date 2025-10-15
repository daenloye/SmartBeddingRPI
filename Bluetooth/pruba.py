import time
import struct
from bluepy import btle
import logging

# Configura el log
logging.basicConfig(level=logging.INFO)

# --- 1. Definición de UUIDs (Identificadores) ---
# UUID del Servicio Personalizado (128 bits)
CUSTOM_SVC_UUID = btle.UUID("12345678-1234-5678-1234-567890abcdef")

# UUID de Característica de Lectura (Read)
READ_CHAR_UUID = btle.UUID("12345678-1234-5678-1234-567890abcd01")

# --- 2. Clase Manejadora de Característica ---
class StatusCharacteristic(btle.Characteristic):
    """
    Define el comportamiento de la característica de lectura.
    """
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Valor inicial (debe ser bytes)
        self.value = b"Hello from Pi Zero" 

    def read(self):
        """Método llamado cuando el cliente móvil solicita leer el valor."""
        current_time = time.strftime("%H:%M:%S").encode('utf-8')
        logging.info(f"Cliente leyendo: {current_time.decode()}")
        # Actualiza el valor con la hora actual antes de enviarlo
        self.value = current_time
        return self.value

# --- 3. Clase Manejadora del Servidor (Delegado) ---
class MyDelegate(btle.DefaultDelegate):
    """Maneja eventos como conexiones y desconexiones."""
    def __init__(self):
        btle.DefaultDelegate.__init__(self)

    def handleNotification(self, cHandle, data):
        # Manejar notificaciones o indicaciones si se implementan
        logging.info(f"Notificación recibida: {data}")

# --- 4. Configuración Principal del Servidor ---
def start_ble_server():
    try:
        logging.info("Inicializando servidor BLE...")
        
        # 4.1. Configuración de Servicio y Característica
        svc = btle.Service(CUSTOM_SVC_UUID)
        
        # Característica con permisos de Lectura (READ)
        char = StatusCharacteristic(
            READ_CHAR_UUID, 
            btle.Characteristic.PROP_READ, 
            btle.Characteristic.PERM_READ,
            svc
        )
        svc.addCharacteristic(char)

        # 4.2. Crear el Peripheral
        # Nota: La librería maneja internamente la ausencia de un pin de emparejamiento
        peripheral = btle.Peripheral(btle.Scanner().getDevice(0))
        peripheral.setDelegate(MyDelegate())
        
        # Iniciar el proceso de advertising
        # Aquí puedes definir el nombre (local_name)
        peripheral.setAdvertData(
            [
                (btle.AD_TYPE_FLAGS, b'\x06'),  # BLE general discoverable
                (btle.AD_TYPE_COMPLETE_LOCAL_NAME, b'PiZeroW_Service')
            ]
        )
        
        # Configuración del Advertising
        peripheral.setServices([svc])
        peripheral.setAdvertising(True)

        logging.info("Servidor 'PiZeroW_Service' corriendo y anunciando. Esperando conexión...")

        while True:
            # Puedes poner aquí otra lógica de la aplicación
            time.sleep(1)

    except Exception as e:
        logging.error(f"Error en el servidor BLE: {e}")
    finally:
        # Detener la publicidad y cerrar al salir
        if 'peripheral' in locals():
            peripheral.setAdvertising(False)
            logging.info("Publicidad detenida. Servidor apagado.")

if __name__ == "__main__":
    start_ble_server()