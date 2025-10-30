from PyQt5.QtCore import QObject, QThread, pyqtSignal, QTimer
import paho.mqtt.client as mqtt
import time
import os
from datetime import datetime
import json
import numpy as np


class MQTTWorker(QThread):
    message_received = pyqtSignal(str, str)
    connected = pyqtSignal()
    disconnected = pyqtSignal()

    def __init__(self, server, port, client_id, password, bedding_id, conn_type, logger=None):
        super().__init__()
        self.server = server
        self.port = port
        self.client_id = client_id
        self.password = password
        self.bedding_id = bedding_id
        self.logger = logger
        self._running = True
        self._client = None
        self.__conn_type=conn_type

    def log(self, func, msg, level=0, error=None):
        """Helper interno para usar el logger si existe"""
        if self.logger:
            self.logger.log(app="MQTTWorker", func=func, level=level, msg=msg, error=error)
        else:
            print(f"[MQTTWorker] {msg}")

    def run(self):
        self._client = mqtt.Client(client_id=f"{self.bedding_id}")

        if(self.__conn_type=="prod"):
            self._client.username_pw_set(self.client_id, self.password)

        self._client.on_connect = self.on_connect
        self._client.on_message = self.on_message
        self._client.on_disconnect = self.on_disconnect

        while self._running:
            try:
                self._client.connect(self.server, self.port, keepalive=60)
                self._client.loop_forever()
            except Exception as e:
                self.log("run", f"Error de conexión: {e}", level=2, error=e)
                time.sleep(5)

    def stop(self):
        self._running = False
        if self._client:
            self._client.disconnect()
            self._client.loop_stop()
        self.log("stop", "Hilo detenido correctamente.")

    def on_connect(self, client, userdata, flags, rc):
        if rc == 0:
            self.log("on_connect", "Conectado correctamente.")
            self.connected.emit()
            client.subscribe(f"sb/response/{self.bedding_id}")
            #client.subscribe(f"sb/init/{self.bedding_id}")
        else:
            self.log("on_connect", f"Error al conectar. Código: {rc}", level=2)

    def on_message(self, client, userdata, msg):
        topic = msg.topic
        payload = msg.payload.decode("utf-8")
        self.message_received.emit(topic, payload)

    def on_disconnect(self, client, userdata, rc):
        self.log("on_disconnect", "Desconectado.")
        self.disconnected.emit()

    def publish(self, topic, payload):
        if self._client and self._client.is_connected():
            result = self._client.publish(topic, payload)
            if result.rc == mqtt.MQTT_ERR_SUCCESS:
                self.log("publish", f"Mensaje publicado correctamente en {topic}")
                return True
            else:
                self.log("publish", f"Fallo al publicar en {topic}, código: {result.rc}", level=2)
                return False
        else:
            self.log("publish", "No conectado. No se puede publicar.", level=2)
            return False


class MQTTManager(QObject):
    message_received = pyqtSignal(str, str)
    connected = pyqtSignal()
    disconnected = pyqtSignal()

    def __init__(self, logger=None):
        super().__init__()
        self.logger = logger
        self.worker = None
        self.queue = []
        self.connection = False

        self.timer = QTimer()
        self.timer.timeout.connect(self.processQueue)
        self.timer.start(30_000)

        self.inicializado = False
        self.clientId = "000001"
        self.initTime = int(time.time())
        self.firstMessage = True

        self.dataStructure = {
            "timestamp": 0,
            "temperature": [],
            "humidity": [],
            "respiratoryRate": [],
            "heartRate": [],
            "heartRateVariability": [],
            "position": [0, 0, 0]
        }

        self.__dbLevel=10

        self.currentData = self.dataStructure.copy()

        os.makedirs("Backups", exist_ok=True)
        today = datetime.now().strftime("%Y-%m-%d")
        existing = [d for d in os.listdir("Backups") if d.startswith(today)]
        backup_name = f"{today}_{len(existing)+1}"
        self.backup_path = os.path.join("Backups", backup_name)

    def log(self, func, msg, level=0, error=None):
        if self.logger:
            self.logger.log(app="MQTTManager", func=func, level=level, msg=msg, error=error)
        else:
            print(f"[MQTTManager] {msg}")

    def start(self):

        conn_type = "prod" # test|prod

        if conn_type == "test":
            server = "192.168.0.109"
            port = 1883
            client_id = ""
            password = ""
        
        else:
            server = "3.90.24.183"
            port = 8807
            client_id = "smartbedding_publisher"
            password = "Sb998?-Tx"
        bedding_id = self.clientId

        self.worker = MQTTWorker(server, port, client_id, password, bedding_id, conn_type, logger=self.logger)
        self.worker.message_received.connect(self.message_received)
        self.worker.connected.connect(self.on_connected)
        self.worker.disconnected.connect(self.on_disconnected)
        self.worker.start()
        self.log("start", "Worker MQTT iniciado.")

    def stop(self):
        if self.worker:
            self.worker.stop()
            self.worker.wait()
            self.log("stop", "Worker MQTT detenido.")

    def on_connected(self):
        self.connection = True
        self.connected.emit()
        self.log("on_connected", "Conectado al broker.")

        if  not self.inicializado:
            self.initMessage()

    def on_disconnected(self):
        self.connection = False
        self.disconnected.emit()
        self.log("on_disconnected", "Desconectado del broker.", level=1)

    def send_message(self, topic, payload):
        """Publica un mensaje en MQTT con manejo de logs y errores."""
        if not self.worker:
            self.logger.log(app="MQTTManager", func="send_message", level=2,
                            msg="No hay worker activo. No se puede enviar mensaje.")
            return False

        try:
            success = self.worker.publish(topic, payload)
            if success:
                self.logger.log(app="MQTTManager", func="send_message", level=0,
                                msg=f"Envío exitoso → {topic} : {payload}")
                if topic == f"sb/record/{self.clientId}":
                    self.firstMessage = False
                return True
            else:
                self.logger.log(app="MQTTManager", func="send_message", level=2,
                                msg=f"Error al enviar mensaje a {topic}")
                return False
        except Exception as e:
            self.logger.log(app="MQTTManager", func="send_message", level=3,
                            msg=f"Excepción al publicar en {topic}", error=e)
            return False


    def add_queue(self):
        self.queue.append(self.currentData.copy())

        self.logger.log(app="MQTTManager", func="add_queue", level=0,
                        msg=f"Se añadió data a la cola. Tamaño actual de la cola: {len(self.queue)}")
        
        self.currentData = self.dataStructure.copy()

    def receivData(self, data):

        self.logger.log(app="MQTTManager", func="receivData", level=0,
                        msg=f"Data recibida en MQTTManager")

        self.currentData["temperature"].append(data["temperature"])
        self.currentData["humidity"].append(data["humidity"])
        self.currentData["respiratoryRate"].append(data["respiratoryRate"])
        self.currentData["heartRate"].append(data["heartRate"])
        self.currentData["heartRateVariability"].append(data["heartRateVariability"])
        self.currentData["position"][data["position"]["final"]["index"]] += 1
        
        if len(self.currentData["temperature"]) >= 1:
            self.add_queue()

    def processQueue(self):
        if not self.connection:
            self.log("processQueue", "Sin conexión, no se procesa nada.", level=1)
            return

        today = datetime.now().strftime("%Y-%m-%d")
        backup_base = "Backups"

        # --- Procesar backups pendientes (solo si ya está inicializado) ---
        if self.inicializado and os.path.exists(backup_base):
            today_folders = sorted(
                [os.path.join(backup_base, d)
                for d in os.listdir(backup_base)
                if d.startswith(today) and os.path.isdir(os.path.join(backup_base, d))]
            )
            for folder in today_folders:
                json_files = sorted([f for f in os.listdir(folder) if f.endswith(".json")])
                if not json_files:
                    continue
                self.log("processQueue", f"Procesando backups pendientes ({len(json_files)} archivos) en {folder}")
                for file in json_files:
                    file_path = os.path.join(folder, file)
                    try:
                        with open(file_path, "r", encoding="utf-8") as f:
                            data_to_send = json.load(f)

                        topic = f"sb/record/{self.clientId}"
                        payload = json.dumps(data_to_send)
                        self.send_message(topic, payload)
                        self.log("processQueue", f" → Backup reenviado y eliminado: {file_path}")
                        os.remove(file_path)
                    except Exception as e:
                        self.log("processQueue", f"Error al reenviar backup {file_path}", level=2, error=e)
                        break
                if not os.listdir(folder):
                    os.rmdir(folder)
                    self.log("processQueue", f"Carpeta de backup vacía eliminada: {folder}")

        # --- Procesar cola ---
        if not self.queue:
            self.log("processQueue", "No hay mensajes en cola para procesar.")
            return

        self.log("processQueue", f"Procesando {len(self.queue)} mensajes en cola...")
        try:
            for data in list(self.queue):
                topic = f"sb/record/{self.clientId}"
                data_to_send = {
                    "s": self.clientId,
                    "init": str(1 if self.firstMessage else 0),
                    "ev": "0",
                    "t": int(time.time()),
                    "data": {
                        "te": str(int(np.mean(data["temperature"]) if data["temperature"] else 0)),
                        "hu": str(int(np.mean(data["humidity"]) if data["humidity"] else 0)),
                        "hf": str(int(np.mean(data["heartRate"]) if data["heartRate"] else 0)),
                        "bf": str(int(np.mean(data["respiratoryRate"]) if data["respiratoryRate"] else 0)),
                        "no": str(int(self.__dbLevel)),
                        "ps": str(max(data["position"]) + 1),
                        "pm": {"00": "0", "01": "0", "02": "0", "10": "0", "11": "0", "12": "0"},
                        "sk": "1",
                        "iq": {}
                    },
                }
                payload = json.dumps(data_to_send)

                # --- Si no está inicializado, guarda backup y espera ---
                if not self.inicializado:
                    self.log("processQueue", "No inicializado → guardando backup y enviando initMessage...", level=1)
                    self.createBackup(data_to_send)
                    self.initMessage()
                    time.sleep(2)
                    continue  # no se envía nada aún

                # --- Envío normal ---
                self.send_message(topic, payload)
                self.log("processQueue", f" → Enviado desde cola: {data['timestamp']}")
                self.queue.remove(data)

            self.log("processQueue", "Todos los mensajes de la cola enviados correctamente.")
        except Exception as e:
            self.log("processQueue", f"Error al enviar cola", level=2, error=e)

    def initMessage(self):
        init_payload = {"s": self.clientId}
        topic = f"sb/init/{self.clientId}"
        payload = json.dumps(init_payload)
        success = self.send_message(topic, payload)
        if success:
            self.log("initMessage", "Mensaje de inicialización enviado correctamente.")
            #self.inicializado = True
        else:
            self.log("initMessage", "Fallo al enviar mensaje de inicialización.", level=2)
            self.inicializado = False

    def setInitialized(self):
        self.inicializado = True
        self.log("setInitialized", "Llega la inicialización desde el broker.")


    def receivAudioDB(self, db):
        self.logger.log(app="MQTTManager", func="receivAudioDB", level=0,
                        msg=f"DB de audio recibido → {db}")
        
        self.__dbLevel=db

    def createBackup(self, data):
        """Guarda un respaldo del mensaje en un archivo JSON dentro de Backups/"""
        try:
            os.makedirs(self.backup_path, exist_ok=True)

            timestamp = datetime.now().strftime("%H-%M-%S_%f")
            file_path = os.path.join(self.backup_path, f"{timestamp}.json")

            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)

            self.log("createBackup", f"Backup creado: {file_path}")
        except Exception as e:
            self.log("createBackup", "Error al crear backup", level=2, error=e)

