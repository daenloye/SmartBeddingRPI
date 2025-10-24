from PyQt6.QtCore import QObject, QThread, pyqtSignal, QTimer
import paho.mqtt.client as mqtt
import time
import os
from datetime import datetime
import json
import numpy as np

class MQTTWorker(QThread):
    message_received = pyqtSignal(str, str)  # topic, payload
    connected = pyqtSignal()
    disconnected = pyqtSignal()

    def __init__(self, server, port, client_id, password, bedding_id):
        super().__init__()
        self.server = server
        self.port = port
        self.client_id = client_id
        self.password = password
        self.bedding_id = bedding_id
        self._running = True
        self._client = None

    def run(self):
        """Hilo principal del worker MQTT"""
        self._client = mqtt.Client(client_id=self.client_id)
        self._client.username_pw_set(self.client_id, self.password)
        self._client.on_connect = self.on_connect
        self._client.on_message = self.on_message
        self._client.on_disconnect = self.on_disconnect

        while self._running:
            try:
                self._client.connect(self.server, self.port, keepalive=60)
                self._client.loop_forever()
            except Exception as e:
                print(f"[MQTT] Error de conexión: {e}")
                time.sleep(5)  # Reintento automático

    def stop(self):
        """Detiene el hilo MQTT"""
        self._running = False
        if self._client:
            self._client.disconnect()
            self._client.loop_stop()

    def on_connect(self, client, userdata, flags, rc):
        if rc == 0:
            print("[MQTT] Conectado correctamente.")
            self.connected.emit()
            client.subscribe(f"sb/response/{self.bedding_id}")
            client.subscribe(f"sb/init/{self.bedding_id}")
        else:
            print(f"[MQTT] Error al conectar. Código: {rc}")

    def on_message(self, client, userdata, msg):
        topic = msg.topic
        payload = msg.payload.decode("utf-8")
        self.message_received.emit(topic, payload)

    def on_disconnect(self, client, userdata, rc):
        print("[MQTT] Desconectado.")
        self.disconnected.emit()

    def publish(self, topic, payload):
        """Publica un mensaje en un topic y confirma si se envió."""
        if self._client and self._client.is_connected():
            result = self._client.publish(topic, payload)
            if result.rc == mqtt.MQTT_ERR_SUCCESS:
                print(f"[MQTT] Mensaje publicado correctamente en {topic}")
                return True
            else:
                print(f"[MQTT] Fallo al publicar en {topic}, código: {result.rc}")
                return False
        else:
            print("[MQTT] No conectado. No se puede publicar.")
            return False


class MQTTManager(QObject):
    message_received = pyqtSignal(str, str)
    connected = pyqtSignal()
    disconnected = pyqtSignal()

    def __init__(self):
        super().__init__()
        self.worker = None

        self.queue = []
        self.connection = False

        # === Timer cada 30 segundos ===
        self.timer = QTimer()
        self.timer.timeout.connect(self.processQueue)
        self.timer.start(30_000)  # 30 segundos

        #Initialized data structure
        self.inicializado=False
        self.clientId="000001"
        self.initTime=int(time.time())
        self.firstMessage=True

        # Estructura base
        self.dataStructure = {
            "timestamp": 0,
            "temperature": [],
            "humidity": [],
            "respiratoryRate": [],
            "heartRate": [],
            "heartRateVariability": [],
            "position": [0, 0, 0]
        }

        self.currentData = self.dataStructure.copy()

        # === Backup si no hay conexión ===
        os.makedirs("Backups", exist_ok=True)
        today = datetime.now().strftime("%Y-%m-%d")
        existing = [d for d in os.listdir("Backups") if d.startswith(today)]
        backup_name = f"{today}_{len(existing)+1}"
        self.backup_path = os.path.join("Backups", backup_name)

    def start(self):
        """Inicia el worker MQTT"""
        server = "3.90.24.183"
        port = 8807
        client_id = "smartbedding_publisher"
        password = "Sb998?-Tx"
        bedding_id=self.clientId

        self.worker = MQTTWorker(server, port, client_id, password, bedding_id)

        # Conecta señales del worker
        self.worker.message_received.connect(self.message_received)
        self.worker.connected.connect(self.on_connected)
        self.worker.disconnected.connect(self.on_disconnected)

        self.worker.start()
        print("[MQTTManager] Worker iniciado.")

    def stop(self):
        """Detiene el worker MQTT"""
        if self.worker:
            self.worker.stop()
            self.worker.wait()
            print("[MQTTManager] Worker detenido.")

    # === Actualización de estado de conexión ===
    def on_connected(self):
        self.connection = True
        print("[MQTTManager] Conectado al broker.")

    def on_disconnected(self):
        self.connection = False
        print("[MQTTManager] Desconectado del broker.")

    # === Envío MQTT ===
    def send_message(self, topic, payload):
        if self.worker:
            success = self.worker.publish(topic, payload)
            if success:
                print(f"[MQTTManager] Envío exitoso → {topic}")
                if topic == f"sb/data/{self.clientId}":
                    self.firstMessage = False
                return True
            else:
                print(f"[MQTTManager] Error al enviar mensaje a {topic}")
                return False
        return False

    # === Manejo de datos y cola ===
    def add_queue(self):
        self.queue.append(self.currentData.copy())
        self.currentData = self.dataStructure.copy()

    def receivData(self, data):
        self.currentData["temperature"].append(data["temperature"])
        self.currentData["humidity"].append(data["humidity"])
        self.currentData["respiratoryRate"].append(data["respiratoryRate"])
        self.currentData["heartRate"].append(data["heartRate"])
        self.currentData["heartRateVariability"].append(data["heartRateVariability"])
        self.currentData["position"][data["position"]["final"]["index"]] += 1

        if len(self.currentData["temperature"]) >= 5:
            self.add_queue()

    # === Procesamiento periódico ===
    def processQueue(self):
        from datetime import datetime

        # Si no hay conexión, no hacemos nada
        if not self.connection:
            print("[Queue] Sin conexión, no se procesa nada.")
            return

        today = datetime.now().strftime("%Y-%m-%d")
        backup_base = "Backups"

        # === 1️⃣ Procesar backups del día de hoy ===
        if os.path.exists(backup_base):
            today_folders = sorted(
                [os.path.join(backup_base, d)
                for d in os.listdir(backup_base)
                if d.startswith(today) and os.path.isdir(os.path.join(backup_base, d))]
            )

            for folder in today_folders:
                json_files = sorted([f for f in os.listdir(folder) if f.endswith(".json")])
                if not json_files:
                    continue

                print(f"[Queue] Procesando backup de hoy: {folder} ({len(json_files)} archivos)")

                for file in json_files:
                    file_path = os.path.join(folder, file)
                    try:
                        with open(file_path, "r", encoding="utf-8") as f:
                            data = json.load(f)

                        # === Simulación de envío (aquí iría el publish real) ===
                        topic = "sb/data/000001"
                        # payload = json.dumps(data)
                        # self.send_message(topic, payload)
                        print(f" → Enviado desde backup: {file_path}")

                        # Si se envió correctamente, eliminar el archivo
                        os.remove(file_path)

                    except Exception as e:
                        print(f"[Backup] Error al procesar {file_path}: {e}")
                        break  # Detenemos si hay fallo en envío para no perder datos

                # Si la carpeta quedó vacía, la eliminamos
                if not os.listdir(folder):
                    os.rmdir(folder)
                    print(f"[Backup] Carpeta vacía eliminada: {folder}")

        # === 2️⃣ Procesar cola en memoria ===
        if len(self.queue) == 0:
            print("[Queue] No hay mensajes en cola para procesar.")
            return

        print(f"[Queue] Procesando {len(self.queue)} mensajes en cola...")

        try:
            for data in list(self.queue):
                topic = "sb/data/000001"

                data_to_send={
                    "init": str(1 if self.firstMessage else 0),
                    "ev":"0",
                    "t":int(time.time()),
                    "var":{
                        "te":str(np.mean(data["temperature"]) if data["temperature"] else 0),
                        "hu":str(np.mean(data["humidity"]) if data["humidity"] else 0),
                        "bf":str(np.mean(data["respiratoryRate"]) if data["respiratoryRate"] else 0),
                        "hf":str(np.mean(data["heartRate"]) if data["heartRate"] else 0),
                        "no":"10",
                        "pos":str(max(data["position"]) + 1),
                        "pm":{
                            "00":"0",
                            "01":"0",
                            "02":"0",
                            "10":"0",
                            "11":"0",
                            "12":"0",
                        },
                        "sk": "1", # 1 Despierto 2 Ligero 3 Profundo 4 REM
                        "iq":{}
                    },

                }
                payload = json.dumps(data_to_send)
                self.send_message(topic, payload)
                print(f" → Enviado desde cola: {data['timestamp']}")
                self.queue.remove(data)

            print(f"[Queue] Todos los mensajes de la cola enviados correctamente.")

        except Exception as e:
            print(f"[Queue] Error al enviar cola: {e}")

    # === Proceso de registro ===
    def initMessage(self):
        init_payload = {"s": self.clientId}
        topic = f"sb/init/{self.clientId}"
        payload = json.dumps(init_payload)
        success = self.send_message(topic, payload)
        if success:
            print("[MQTTManager] Mensaje de inicialización enviado correctamente.")
            self.inicializado = True
        else:
            print("[MQTTManager] Fallo al enviar mensaje de inicialización.")
            self.inicializado = False