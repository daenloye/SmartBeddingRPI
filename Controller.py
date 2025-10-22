#

import sys
import asyncio
import signal

from PyQt5.QtCore import QCoreApplication
from qasync import QEventLoop

from LoggerManager import Logger
from EnvironmentAdquisition import EnvironmentManager
from PressureAdquisition import PressureReader
from AccelerationAdquisition import AccelerationReader
from Model import Model
from MQTTManager import MQTTManager
from AudioRecorder import AudioRecorder

class Controlador:
    def __init__(self):
        # -----------------------------------------
        # Aplicación Qt (sin GUI, solo loop de eventos)
        # -----------------------------------------
        self.app = QCoreApplication(sys.argv)

        # -----------------------------------------
        # Loop de asyncio integrado con Qt
        # -----------------------------------------
        self.loop = QEventLoop(self.app)
        asyncio.set_event_loop(self.loop)

        # -----------------------------------------
        # Logger
        # -----------------------------------------
        self.logger = Logger()

        # -----------------------------------------
        # Flag de almacenar archivos de depuración
        # -----------------------------------------

        self.debugFiles=True

        # -----------------------------------------
        # Modelo de datos
        # -----------------------------------------

        self.model = Model(self, self.logger,self.debugFiles)

        # -----------------------------------------
        # Reporte de datos
        # -----------------------------------------

        self.mqtt=MQTTManager()
        self.mqtt.message_received.connect(self.on_msg_mqtt)

        # -----------------------------------------
        # Ambiente
        # -----------------------------------------
        self.environment = EnvironmentManager(interval=20_000, max_samples=3, logger=self.logger)  # cada 20 s, 3 muestras
        self.environment.new_sample.connect(self.on_env_sample)
        self.environment.start()          # inicializa y prepara el hilo

        # -----------------------------------------
        # Presión
        # -----------------------------------------
        self.pressure = PressureReader(loop=self.loop, logger=self.logger)
        self.pressure.start()          # inicializa y prepara el hilo

        # -----------------------------------------
        # Aceleración/Giroscopio
        # -----------------------------------------
        self.acceleration = AccelerationReader(interval=0.05)  # 20 Hz

        # -----------------------------------------
        # Sonido
        # -----------------------------------------

        self.audio = AudioRecorder(self.logger,self.model.getCurrentFolder(), duration=60,samplerate=44100, channels=1)
        self.audio.start()

        # -----------------------------------------
        # Flags de Status
        # -----------------------------------------

        self.muestreando=False # Si está muestreando

    async def quit(self, sig=None):
        self.logger.log(app="Controlador", func="quit", level=0,
                        msg=f"Ha llegado la señal de salida ({sig})")
        
        self.pressure.shutdown() # detiene el muestreo y cierra el hilo
        self.acceleration.stop()  # detiene el muestreo y cierra el hilo

        tasks = [t for t in asyncio.all_tasks() if t is not asyncio.current_task()]
        [t.cancel() for t in tasks]

        self.logger.log(app="Controlador", func="quit", level=0,
                        msg=f"Cancelando {len(tasks)} tareas pendientes...")

        await asyncio.gather(*tasks, return_exceptions=True)
        self.loop.stop()

    def run(self):
        # Registrar manejadores de señales (Linux/Mac, no Windows)
        try:
            for sig in (signal.SIGINT, signal.SIGTERM):
                self.loop.add_signal_handler(sig, lambda s=sig: self.loop.create_task(self.quit(s)))
        except NotImplementedError:
            # En Windows no está implementado
            pass

        # Ejecutar el bucle de eventos
        with self.loop:
            try:
                self.loop.create_task(self.start())
                self.loop.run_forever()
            except Exception as e:
                self.logger.log(app="Controlador", func="run", level=2,
                                msg="Error inesperado", error=e)
            finally:
                self.logger.log(app="Controlador", func="Close", level=0,
                                msg="Cerrando sistema")
                self.loop.close()

    async def start(self):
        self.logger.log(app="Controlador", func="start", level=0,
                        msg="Iniciando lógica principal")
        
        # Conectar la señal del worker a un método local
        self.pressure.worker.new_sample.connect(self.on_new_pressure)

        self.acceleration.worker.new_sample.connect(self.on_new_acceleration)
        
        self.pressure.begin_sampling()  # comienza el muestreo de presión
        self.acceleration.start()        # comienza el muestreo de aceleración/giroscopio


    def on_new_pressure(self, timestamp,matrix):
        #Enviar a almacenar
        self.model.storePressure(timestamp, matrix)
        
    def on_new_acceleration(self, timestamp,matrix):

        #Enviar a almacenar
        self.model.storeAcceleration(timestamp, matrix)


    def on_env_sample(self, timestamp, temperature, humidity):
        #Enviar a almacenar
        self.model.storeEnvironment(timestamp, temperature, humidity)

    # -----------------------------------------
    # Métodos para enviar a MQTT
    # -----------------------------------------


    def on_msg_mqtt(self,topic, payload):
        print(f"[Mensaje] {topic}: {payload}")

    def send_to_mqtt(self,data):
        pass

if __name__ == "__main__":
    c = Controlador()
    c.run()
