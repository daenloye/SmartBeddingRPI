#

import sys
import asyncio
import signal

from PyQt5.QtCore import QCoreApplication, QTimer
from qasync import QEventLoop
import json
import time


from LoggerManager import Logger
from EnvironmentAdquisition import EnvironmentManager
from PressureAdquisition import PressureReader
from AccelerationAdquisition import AccelerationReader
from Model import Model
from MQTTManager import MQTTManager
from AudioRecorder import AudioRecorder
import subprocess

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

        self.mqtt = MQTTManager(logger=self.logger)
        self.mqtt.message_received.connect(self.on_msg_mqtt)
        self.mqtt.start()

        # -----------------------------------------
        # Ambiente
        # -----------------------------------------
        self.environment = EnvironmentManager(interval=20_000, max_samples=3, logger=self.logger)  # cada 20 s, 3 muestras
        self.environment.new_sample.connect(self.on_env_sample)
        self.environment.start()          # inicializa y prepara el hilo

        # -----------------------------------------
        # Presión
        # -----------------------------------------
        self.pressure = PressureReader(interval=1.0)
        self.pressure.start()          # inicializa y prepara el hilo

        # -----------------------------------------
        # Aceleración/Giroscopio
        # -----------------------------------------
        self.acceleration = AccelerationReader(interval=0.05)  # 20 Hz

        # -----------------------------------------
        # Sonido
        # -----------------------------------------

        self.audio = AudioRecorder(self,self.logger,self.model.getCurrentFolder(), duration=60,samplerate=44100, channels=1)
        self.audio.start()

        # -----------------------------------------
        # Flags de Status
        # -----------------------------------------

        self.muestreando=False # Si está muestreando

    async def quit(self, sig=None):
        self.logger.log(app="Controlador", func="quit", level=0,
                        msg=f"Ha llegado la señal de salida ({sig})")
        
        self.pressure.stop() # detiene el muestreo y cierra el hilo
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
        self.pressure.new_sample.connect(self.on_new_pressure)

        self.acceleration.new_sample.connect(self.on_new_acceleration)
        
        self.pressure.start()  # comienza el muestreo de presión
        self.acceleration.start()        # comienza el muestreo de aceleración/giroscopio

        #Inicializo el timer

        # # Iniciar timer (60 segundos)
        # self.timer = QTimer(self)
        # self.timer.timeout.connect(self.on_tick)
        # self.timer.start(60_000)


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


    def on_msg_mqtt(self, topic, payload):
        self.logger.log(app="Controller", func="on_msg_mqtt", level=0,
                        msg=f"Mensaje recibido - {topic}: {payload}")
        try:
            data = json.loads(payload)
        except Exception as e:
            self.logger.log(app="Controller", func="on_msg_mqtt", level=2,
                            msg="Error parseando JSON", error=e)
            return

        #Analizo si llega error o no
        if data.get("error", None) is not None:
            self.logger.log(app="Controller", func="on_msg_mqtt", level=2,
                            msg=f"Error recibido desde el cliente: {data['error']}")
            return
        else:
            self.logger.log(app="Controller", func="on_msg_mqtt", level=0,
                            msg=f"Se recibe mensaje corretamente sin errores")
            
        # Si lleg el time
        if data.get("t") is not None:
            try:
                new_time = int(data["t"])  # asegurar tipo entero
                if new_time <= 0:
                    raise ValueError("timestamp inválido o negativo")
                

                if False:
                    # Ajustar el reloj del sistema (requiere permisos sudo para date)
                    subprocess.run(["sudo", "date", "-s", f"@{new_time}"], check=True)

                    self.logger.log(
                        app="Controller",
                        func="on_msg_mqtt",
                        level=0,
                        msg=f"🕒 Reloj del sistema ajustado a {new_time} (Unix time)"
                    )

                #Envio la inicialización
                self.mqtt.setInitialized()

            except Exception as e:
                self.logger.log(
                    app="Controller",
                    func="on_msg_mqtt",
                    level=2,
                    msg="Error ajustando el reloj del sistema",
                    error=e
                )
        
        #Si llega el side
        if data.get("side", None) is not None:
            side = data["side"]
            self.model.setSide(side)

            #Envio la inicialización
            self.mqtt.setInitialized()


    def receivMqttData(self, data):
        self.mqtt.receivData(data)

    
    def receivAudioDB(self, db):
        self.mqtt.receivAudioDB(db)

if __name__ == "__main__":
    c = Controlador()
    c.run()
