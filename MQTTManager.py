from PyQt6.QtCore import QObject, QThread, pyqtSignal, QTimer
import paho.mqtt.client as mqtt
import time
import os
from datetime import datetime
import json

class MQTTWorker(QThread):
    message_received = pyqtSignal(str, str)  # topic, payload
    connected = pyqtSignal()
    disconnected = pyqtSignal()

    def __init__(self, server, port, client_id, password, topic_sub):
        super().__init__()
        self.server = server
        self.port = port
        self.client_id = client_id
        self.password = password
        self.topic_sub = topic_sub
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
            client.subscribe(self.topic_sub)
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
        """Publica un mensaje en un topic"""
        if self._client and self._client.is_connected():
            self._client.publish(topic, payload)
        else:
            print("[MQTT] No conectado. No se puede publicar.")


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
        topic_sub = "sb/response/000001"

        self.worker = MQTTWorker(server, port, client_id, password, topic_sub)

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
            self.worker.publish(topic, payload)

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
                # payload = json.dumps(data)
                # self.send_message(topic, payload)
                print(f" → Enviado desde cola: {data['timestamp']}")
                self.queue.remove(data)

            print(f"[Queue] Todos los mensajes de la cola enviados correctamente.")

        except Exception as e:
            print(f"[Queue] Error al enviar cola: {e}")

