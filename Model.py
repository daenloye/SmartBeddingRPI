
import numpy as np
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal
import os
import json
from scipy.signal import lfilter, filtfilt, detrend
import psutil

# Coeficientes del filtro
b_rrs = [4.975743576868226e-05, 0.0, -0.00014927230730604678, 0.0,
         0.00014927230730604678, 0.0, -4.975743576868226e-05]

a_rrs = [1.0, -5.830766569820652, 14.185404142052889, -18.43141872929975,
         13.489689338789688, -5.2728999261646115, 0.8599919781204693]

b_crs = [0.0010739281487746567, 0.0, -0.004295712595098627, 0.0, 
         0.006443568892647941, 0.0, -0.004295712595098627, 0.0, 0.0010739281487746567]

a_crs = [1.0, -6.4557706152374905, 18.656818730243238, -31.516992353914958, 34.03663934201975, 
         -24.062919294682047, 10.877684610556427, -2.8761856141583015, 0.34094015209888484]

#------------------------------------------------------
# Worker: procesa y guarda un registro en segundo plano
#------------------------------------------------------
class RecordWorker(QObject):
    finished = pyqtSignal()

    def __init__(self, record, folder, id, logger,debug):
        super().__init__()
        self.record:MinuteRecord = record
        self.logger = logger
        self.id=id
        self.folder = folder
        self.debug=debug

    def run(self):
        try:
            # Log de inicio
            self.logger.log(app="Modelo", func="RecordWorker", level=0,
                            msg=f"Procesando registro de {self.record.initTimestamp} a {self.record.finishTimestamp}")

            # ------------------------------------------
            # Procesamiento de datos
            # ------------------------------------------

            # El formato es: [gx, gy, gz, ax, ay, az]
            acel_raw=np.array([d['measure'] for d in self.record.acelerationData])
            acel_filtered_rrs = acel_raw.copy()
            acel_filtered_crs = acel_raw.copy()

            # Se aplica el filtro para frecuencia respiratoria solo a gx, gy y gz, que son las 3 primeras columnas
            for i in range(3):
                acel_filtered_rrs[:, i] = filtfilt(b_rrs, a_rrs, acel_raw[:, i])
                acel_filtered_crs[:, i] = filtfilt(b_crs, a_crs, acel_raw[:, i])

            # Calculo la señal RRS
            RRS=(0.7)*acel_filtered_rrs[:,0]+(0.22)*acel_filtered_rrs[:,1]+(0.0775)*acel_filtered_rrs[:,2]
            RRS_detrended=detrend(RRS)

            # Calculo la señal CRS
            CRS=(0.54633)*acel_filtered_crs[:,0]+(0.31161)*acel_filtered_crs[:,1]+(0.15108)*acel_filtered_crs[:,2]


            # ------------------------------------------
            # Medición de rendimiento
            # ------------------------------------------

            # --- CPU ---
            cpu_percent = psutil.cpu_percent(interval=1)   # Porcentaje de uso total
            cpu_cores = psutil.cpu_count(logical=True)     # Núcleos lógicos
            cpu_freq = psutil.cpu_freq()                   # Frecuencia actual, min, max

            # --- Memoria RAM ---
            mem = psutil.virtual_memory()
            mem_total = mem.total / (1024 ** 3)            # en GB
            mem_available = mem.available / (1024 ** 3)
            mem_used = mem.used / (1024 ** 3)
            mem_percent = mem.percent

            # ----------------------------------
            # Almacenamiento
            # ----------------------------------

            # Almaceno el JSON si está en modo debug
            if self.debug:
                data = {
                    "initTimestamp": self.record.initTimestamp,
                    "finishTimestamp": self.record.finishTimestamp,
                    "dataRaw": {
                        "pressure": [
                            {"timestamp": d["timestamp"], "measure": d["measure"].tolist()}
                            for d in self.record.pressureData
                        ],
                        "acceleration": [
                            {"timestamp": d["timestamp"], "measure": d["measure"].tolist()}
                            for d in self.record.acelerationData
                        ],
                        "environment": self.record.environmentData
                    },
                    "dataProcessed": {
                        "RRS": RRS_detrended.tolist(),
                        "CRS": CRS.tolist()
                    },
                    "measures": {
                        "respiratoryRate": None,
                        "heartRate": None,
                        "movementIndex": None,
                        "position": None
                    },
                    "performance": {
                        "cpu": {
                            "percent": cpu_percent,
                            "cores": cpu_cores,
                            "freq_current": cpu_freq.current,
                            "freq_min": cpu_freq.min,
                            "freq_max": cpu_freq.max
                        },
                        "memory": {
                            "total_GB": mem_total,
                            "available_GB": mem_available,
                            "used_GB": mem_used,
                            "percent": mem_percent
                        }
                    }
                }

                with open(os.path.join(self.folder,f"reg_{self.id}.json"),"w") as f:
                    json.dump(data, f, ensure_ascii=False)

            # Log de fin
            self.logger.log(app="Modelo", func="RecordWorker", level=0,
                            msg=f"Registro {self.record.initTimestamp} procesado correctamente")

        except Exception as e:
            self.logger.log(app="Modelo", func="RecordWorker", level=3,
                            msg=f"Error procesando registro: {str(e)}")

        finally:
            self.finished.emit()

#------------------------------------------------------
# DataObject: Almacena la información
#------------------------------------------------------

class MinuteRecord:
    def __init__(self):
        self.pressureData = []  # Lista para almacenar los datos de presión
        self.acelerationData = []  # Lista para almacenar los datos de aceleración
        self.environmentData = []  # Diccionario para almacenar la última muestra de ambiente
        self.initTimestamp=0
        self.finishTimestamp=0

    def storePressure(self, timestamp, pressure):
        self.pressureData.append({'timestamp': timestamp, 'measure': pressure})

    def storeAcceleration(self, timestamp, acceleration):
        self.acelerationData.append({'timestamp': timestamp, 'measure': acceleration})

    def storeEnvironment(self, timestamp, temperature, humidity):
        self.environmentData.append({'timestamp': timestamp, 'temperature': temperature, 'humidity': humidity})

    def checkFull(self):
        return len(self.pressureData) >= 60 and len(self.acelerationData) >= 60*20

#------------------------------------------------------
# Clase: Gestiona lo relacionado con el manejo de la información
#------------------------------------------------------

class Model(QObject):
    def __init__(self,logger=None,degugFiles=False):
        super().__init__()

        self.logger = logger
        self.debugFiles=degugFiles

        self.currentRecord = None  # Variable que almacena el registro actual
        self.lastRecord = None     # Variable que almacena el último registro guardado

        # Inicializo los archivos
        self.idCurrentRecord = 1
        self.currentFolder = ""
        self.initStore()

        #Variable de thread
        self.thread = []

    #-----------------------------------------
    # Método para inicializar el almacenamiento
    #-----------------------------------------

    def initStore(self):
        # Creo la carpeta si no existe
        os.makedirs("DataStorage", exist_ok=True)

        # Cuento cuántos registros hay, contaré carpetas que empiecen con "record_"
        numRegs = len([
            name for name in os.listdir("DataStorage")
            if os.path.isdir(os.path.join("DataStorage", name)) and name.startswith("record_")
        ])

        #Creo el nuevo registro
        self.currentFolder=f"DataStorage/record_{numRegs+1}"
        os.makedirs(self.currentFolder, exist_ok=True)

    #-----------------------------------------
    # Método para almacenar los registros de aceleración y presión
    #-----------------------------------------

    def storeEnvironment(self, timestamp, temperature, humidity):
        if self.currentRecord is not None:
            self.currentRecord.storeEnvironment(timestamp, temperature, humidity)

    def storePressure(self, timestamp, pressure):
        if self.currentRecord is None:
            self.currentRecord = MinuteRecord()
            self.currentRecord.initTimestamp = timestamp

        self.currentRecord.storePressure(timestamp, pressure)

        #Analizo si debo continuar con el registro actual o guardar y crear uno nuevo
        if self.currentRecord.checkFull():
            self.startNextRecord(timestamp)

    def storeAcceleration(self, timestamp, acceleration):
        if self.currentRecord is None:
            self.currentRecord = MinuteRecord()
            self.currentRecord.initTimestamp = timestamp

        self.currentRecord.storeAcceleration(timestamp, acceleration)

        #Analizo si debo continuar con el registro actual o guardar y crear uno nuevo
        if self.currentRecord.checkFull():
            self.startNextRecord(timestamp)

    def startNextRecord(self, timestamp):
        self.currentRecord.finishTimestamp = timestamp #Almacena el cierre del archivo
        # self.lastRecord = self.currentRecord #Lo pasa a lastRecord

        # Lanza el procesamiento en segundo plano
        thread = QThread()
        worker = RecordWorker(self.currentRecord, self.currentFolder, self.idCurrentRecord, self.logger,self.debugFiles)
        worker.moveToThread(thread)

        thread.started.connect(worker.run)
        worker.finished.connect(thread.quit)
        worker.finished.connect(worker.deleteLater)
        thread.finished.connect(thread.deleteLater)

        # Cuando el hilo termine, eliminarlo de la lista
        def remove_thread():
            if thread in self.thread:
                self.thread.remove(thread)
                self.logger.log(app="Modelo", func="startNextRecord", level=1,
                                msg=f"Hilo de RecordWorker finalizado y eliminado ({len(self.thread)} restantes)")

        thread.finished.connect(remove_thread)

        thread.start()

        self.thread.append(thread)  # Mantener una referencia al hilo
        
        #Crear uno nuevo
        self.currentRecord = MinuteRecord()
        self.currentRecord.initTimestamp = timestamp

        #Emito el log
        self.logger.log(app="Modelo", func="startNextRecord", level=0,
                        msg=f"Se inicia un nuevo registro con timestamp {timestamp}")
    

        self.idCurrentRecord += 1

