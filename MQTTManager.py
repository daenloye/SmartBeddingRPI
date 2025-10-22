from PyQt6.QtCore import QObject, QThread, pyqtSignal
import paho.mqtt.client as mqtt
import time

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
    """Administra el hilo MQTT y provee interfaz para enviar mensajes"""
    message_received = pyqtSignal(str, str)
    connected = pyqtSignal()
    disconnected = pyqtSignal()

    def __init__(self):
        super().__init__()
        self.worker = None

        self.queue=[]

        self.dataStructure={
            "timestamp": 0,

            "temperature":[],
            "humidity":[],

            "respiratoryRate": [],
            "heartRate": [],
            "heartRateVariability":[],

            "position": [0,0,0] 
        }

        self.currentData=self.dataStructure.copy()

    def start(self):
        """Inicia el worker MQTT"""
        server = "3.90.24.183"
        port = 8807
        client_id = "smartbedding_publisher"
        password = "Sb998?-Tx"
        topic_sub = "sb/response/000001"

        self.worker = MQTTWorker(server, port, client_id, password, topic_sub)

        # Conecta señales
        self.worker.message_received.connect(self.message_received)
        self.worker.connected.connect(self.connected)
        self.worker.disconnected.connect(self.disconnected)

        self.worker.start()
        print("[MQTTManager] Worker iniciado.")

    def stop(self):
        """Detiene el worker MQTT"""
        if self.worker:
            self.worker.stop()
            self.worker.wait()
            print("[MQTTManager] Worker detenido.")

    def send_message(self, topic, payload):
        """Envía un mensaje MQTT"""
        if self.worker:
            self.worker.publish(topic, payload)

    def add_queue(self):

        #Añado el mensaje al a cola
        self.add_queue.append(self.currentData.copy())

        #Reescribo la estructura
        self.currentData=self.dataStructure.copy()

    def receivData(self,data):
        #Almaceno la data de ambiente
        self.currentData["temperature"].append(data["temperature"])
        self.currentData["humidity"].append(data["humidity"])

        #Almaceno la data que muestreo
        self.currentData["respiratoryRate"].append(data["respiratoryRate"])
        self.currentData["heartRate"].append(data["heartRate"])
        self.currentData["heartRateVariability"].append(data["heartRateVariability"])

        #Almaceno los datos de posición
        self.currentData["position"][data["position"]["final"]["index"]]+=1

        #Envio si se cumple
        if(len(self.currentData["temperature"])>=5):
            self.add_queue()


