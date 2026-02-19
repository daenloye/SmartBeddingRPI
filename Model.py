
import numpy as np
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal
import os
import json
from scipy.signal import lfilter, filtfilt, detrend, find_peaks
import psutil
from datetime import datetime
import copy
import time
from collections import deque

from PositionModel import procesarMuestra

#Eliminar los warnings

import warnings
warnings.filterwarnings("ignore", category=UserWarning)
warnings.filterwarnings("ignore", category=FutureWarning)
warnings.filterwarnings("ignore", category=RuntimeWarning)
warnings.filterwarnings("ignore", category=ImportWarning)
warnings.filterwarnings("ignore", category=DeprecationWarning)

# También suprime las advertencias específicas de sklearn
from sklearn.exceptions import InconsistentVersionWarning
warnings.filterwarnings("ignore", category=InconsistentVersionWarning)

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

    def __init__(self, controlador, record, folder, id, logger,debug,position="R"):
        super().__init__()
        self.controlador=controlador
        self.record:MinuteRecord = record
        self.logger = logger
        self.id=id
        self.folder = folder
        self.debug=debug
        self.position=position

    def calculate_resp_freq_zero_cross(self,signal, sample_restriction, slope_window=5):

        signal = np.asarray(signal).flatten()

        time=np.arange(len(signal)) * 0.05,  # vector de tiempos (20 Hz)

        time = np.asarray(time).flatten()
        if len(signal) != len(time):
            raise ValueError("signal y time deben tener la misma longitud")

        # 1. Detección de cruces por cero (sin wrap-around)
        zx = np.where(signal[:-1] * signal[1:] <= 0)[0]  # índices k donde hay cruce entre k y k+1

        # 2. Eliminar cruces demasiado cercanos
        if zx.size == 0:
            return np.nan
        cross_values = [zx[0]]
        for i in range(1, len(zx)):
            if abs(zx[i] - cross_values[-1]) > sample_restriction:
                cross_values.append(zx[i])
        cross_values = np.array(cross_values, dtype=int)

        if len(cross_values) < 3:
            return np.nan

        # 3. Calcular pendientes (polaridades)
        polarities = np.zeros(len(cross_values), dtype=float)
        for i, idx in enumerate(cross_values):
            # idx es un índice k (cruce entre k y k+1), aproximamos pendiente según ventana
            if idx - slope_window > 0 and idx + slope_window < len(signal):
                slope = signal[idx + slope_window] - signal[idx - slope_window]
            elif idx - slope_window <= 0 and idx + slope_window < len(signal):
                slope = signal[idx + slope_window] - signal[idx]
            elif idx + slope_window >= len(signal) and idx - slope_window > 0:
                slope = signal[idx] - signal[idx - slope_window]
            else:
                slope = np.nan
            polarities[i] = (slope > 0) if not np.isnan(slope) else np.nan

        # 4. Contar respiraciones válidas (evitando "doble disparo")
        resp_count = 0
        is_valid = np.ones(len(cross_values), dtype=bool)
        i = 2  # empezamos desde el tercer cruce (índice 2 en python)
        first_resp_index = None

        while i < len(cross_values):
            # comparar polaridad actual con la anterior
            # si alguna es nan, consideramos inválido y lo marcamos
            if np.isnan(polarities[i]) or np.isnan(polarities[i-1]):
                is_valid[i] = False
                i += 1
                continue

            if polarities[i] != polarities[i-1]:
                if first_resp_index is None:
                    first_resp_index = i - 2
                resp_count += 1
                i += 2  # salto dos cruces (una respiración completa)
            else:
                # mismo signo consecutivo -> posible "doble disparo": eliminar i
                is_valid[i] = False
                i += 1

        valid_crosses = cross_values[is_valid]
        valid_polarities = polarities[is_valid]

        if first_resp_index is None or len(valid_crosses) < 3:
            return np.nan

        try:
            mapped_first = np.where(valid_crosses == cross_values[first_resp_index])[0][0]
        except Exception:
            # si no se puede mapear, fallback: usar el primer índice válido
            mapped_first = 0

        if valid_polarities[mapped_first] != valid_polarities[-1]:
            last_resp_index = valid_crosses[-2]
        else:
            last_resp_index = valid_crosses[-1]

        # 6. Cálculo de duración y frecuencia
        range_time = time[last_resp_index] - time[valid_crosses[0]]
        if range_time <= 0:
            return np.nan

        resp_freq = resp_count * 60.0 / range_time

        return resp_freq

    def calculate_heart_rate_peaks(self, signal, min_peak_height, min_peak_distance, fs=20):
        # Construir vector de tiempo según la frecuencia de muestreo
        time_seconds = np.arange(len(signal)) / fs

        # Convertir min_peak_distance (segundos) a número de muestras
        dt = np.mean(np.diff(time_seconds))
        min_samples = int(min_peak_distance / dt)

        # Detección de picos
        peaks, _ = find_peaks(signal, height=min_peak_height, distance=min_samples)
        pk = signal[peaks]
        lk = time_seconds[peaks]

        # Cálculo de frecuencia cardíaca
        if len(lk) < 2:
            return 0, 0
        
        diff_time = lk[-1] - lk[0]

        if diff_time <= 0 or len(pk) < 2:
            heart_rate = 0
            heart_rate_variability = 0
        else:
            heart_rate = len(pk[1:]) * 60 / diff_time  # bpm
            heart_rate_variability = np.mean(np.diff(lk))

        return heart_rate, heart_rate_variability

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
            # Estimación de posición del minuto
            # ------------------------------------------

            pred_vacio=0
            pred_latDer=0
            pred_latIzq=0
            pred_supino=0

            presSamples=[ d["measure"].tolist() for d in self.record.pressureData ]


            for presSample in presSamples:
                try:
                    prediccion=procesarMuestra(presSample,self.position)
                except Exception as E:
                    self.logger.log(
                        app="Modelo",
                        func="RecordWorker",
                        level=3,
                        msg=f"Error procesando la matriz: {presSample}",
                        error=E
                    )

                if prediccion==0:
                    pred_latIzq+=1
                elif prediccion==1:
                    pred_latDer+=1
                elif prediccion==2:
                    pred_supino+=1
                else:
                    pred_vacio+=1

            #Analizo cual es la posición mayoritaria
            mayor=max(pred_vacio,pred_latDer,pred_latIzq,pred_supino)

            posIndex=-1
            
            if mayor==pred_vacio:
                posicion="Vacio"

            elif mayor==pred_latDer:
                posicion="Lateral Derecho"
                posIndex=1
            elif mayor==pred_latIzq:
                posicion="Lateral Izquierdo"
                posIndex=2
            else:
                posicion="Supino"
                posIndex=3

            # ------------------------------------------
            # Estimación de frecuencia respiratoria
            # ------------------------------------------

            RRS_freq = self.calculate_resp_freq_zero_cross(
                signal=RRS_detrended,
                sample_restriction=20,  # mínimo 1 s entre cruces
                slope_window=5
            )

            self.logger.log(app="Modelo", func="RecordWorker", level=0,
                            msg=f"RR calculada correctamente")


            # ------------------------------------------
            # Estimación de frecuencia cardiaca y variabilidad
            # ------------------------------------------

            HR,HRV=self.calculate_heart_rate_peaks(
                signal=CRS,
                min_peak_height=0,
                min_peak_distance=0.5
            )


            self.logger.log(app="Modelo", func="RecordWorker", level=0,
                            msg=f"HR y HRV calculadas correctamente")

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
                        "respiratoryRate": RRS_freq,
                        "heartRate": HR,
                        "heartRateVariability":HRV,
                        "movementIndex": None,
                        "position": {
                            "estimations":{
                                "Vacio": pred_vacio,
                                "Lateral Derecho": pred_latDer,
                                "Lateral Izquierdo": pred_latIzq,
                                "Supino": pred_supino
                            },
                            "final": {
                                "name": posicion,
                                "index": posIndex
                            }
                        }
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

            # ------------------------------------------
            # Reporte MQTT
            # ------------------------------------------

            MQTT_Report={
                "initTimestamp": self.record.initTimestamp,
                "finishTimestamp": self.record.finishTimestamp,
                "temperature":np.mean([d["temperature"] for d in self.record.environmentData]),
                "humidity":np.mean([d["humidity"] for d in self.record.environmentData]),
                "respiratoryRate": RRS_freq,
                "heartRate": HR,
                "heartRateVariability":HRV,
                "movementIndex": None,
                "position": {
                    "estimations":{
                        "Vacio": pred_vacio,
                        "Lateral Derecho": pred_latDer,
                        "Lateral Izquierdo": pred_latIzq,
                        "Supino": pred_supino
                    },
                    "final": {
                        "name": posicion,
                        "index": posIndex
                    }
                } 
            }

            self.logger.log(app="Modelo", func="RecordWorker", level=0,
            msg=f"Data enviada a MQTTManager")
            
            self.controlador.receivMqttData(MQTT_Report)

            # Log de fin
            self.logger.log(app="Modelo", func="RecordWorker", level=0,
                            msg=f"Registro {self.record.initTimestamp} #{self.id} procesado correctamente")

        except Exception as e:
            self.logger.log(app="Modelo", func="RecordWorker", level=3,
                            msg=f"Error procesando registro #{self.id}: {str(e)}")

class RecordProcesser(QThread):
    # Definimos señales para comunicarnos con la interfaz
    progress = pyqtSignal(int)
    finished_task = pyqtSignal(str)

    def __init__(self, logger):
        super().__init__()
        self.__running=True
        self.logger = logger

        #Defino la cola
        self.queue=deque()

    def addRecord(self,record:RecordWorker):
        self.queue.append(record)

    def stop(self):
        self.__running=False

    def run(self):
        # Aquí va el código que toma mucho tiempo
        while self.__running:
            if len(self.queue)>0:
                #Obtengo el registro
                record:RecordWorker=self.queue.popleft()

                #Lo proceso
                record.run()

                #Espero un tiempo para no saturar
                time.sleep(2)

                self.logger.log(app="Modelo", func="RecordProcesser", level=0,
                msg=f"Datos procesados, ({len(self.queue)} restantes)")
            else:
                #Espero un tiempo prudencial para no saturar el sistema
                time.sleep(10)

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

    def clone(self):
        new = MinuteRecord()
        new.pressureData = copy.deepcopy(self.pressureData)
        new.acelerationData = copy.deepcopy(self.acelerationData)
        new.environmentData = copy.deepcopy(self.environmentData)
        new.initTimestamp = self.initTimestamp
        new.finishTimestamp = self.finishTimestamp
        return new 

    # def checkFull(self, currentTimestamp):
    #     """Devuelve True si ya pasó 1 minuto desde el inicio del registro"""
    #     if self.initTimestamp == 0:
    #         return False

    #     fmt = "%H:%M:%S.%f"

    #     current = datetime.strptime(currentTimestamp, fmt)
    #     init = datetime.strptime(self.initTimestamp, fmt)

    #     elapsed = (current - init).total_seconds()

    #     return elapsed >= self.duration

#------------------------------------------------------
# Clase: Gestiona lo relacionado con el manejo de la información
#------------------------------------------------------

class Model(QObject):
    def __init__(self,controlador, logger=None,degugFiles=False):
        super().__init__()


        self.controlador=controlador
        self.logger = logger
        self.debugFiles=degugFiles

        self.currentRecord = None  # Variable que almacena el registro actual
        # self.lastRecord = deque()     # Variable que almacena el último registro guardado

        # Inicializo los archivos
        self.idCurrentRecord = 1
        self.currentFolder = ""
        self.initStore()

        self.side ="R" # Lado por defecto

        self.duration = 1000*60  # duración deseada en segundos

        #Genero un timer
        self.timer = QTimer(self)
        self.timer.setInterval(self.duration)  # milisegundos
        self.timer.timeout.connect(self.startNextRecord)

        #Inicializo el hilo
        self.record_processor = RecordProcesser(self.logger)
        self.record_processor.start()


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

    def storeAcceleration(self, timestamp, acceleration):
        if self.currentRecord is None:
            self.currentRecord = MinuteRecord()
            self.currentRecord.initTimestamp = timestamp

        self.currentRecord.storeAcceleration(timestamp, acceleration)

    def initializeNewRecord(self):

        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]
        self.currentRecord = MinuteRecord()
        self.currentRecord.initTimestamp = timestamp

        #Inicio el timer
        self.timer.start()

    def startNextRecord(self):
        #Almaceno el timestamp actual
        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]

        #Almaceno el cierre del registro actual
        self.currentRecord.finishTimestamp = timestamp

         #Lo pasa a lastRecord
        # self.lastRecord.append(self.currentRecord)

        #Saco una copia de lastRecord
        record_copy = self.currentRecord.clone()

        #Genero el objeto que almacenará
        worker = RecordWorker(self.controlador, record_copy, self.currentFolder, self.idCurrentRecord, self.logger,self.debugFiles,self.side)

        #Lo agrego a la cola del procesador
        self.record_processor.addRecord(worker)
        
        #Crear uno nuevo
        self.currentRecord = MinuteRecord()

        #Almaceno el timestamp de inicio
        self.currentRecord.initTimestamp = timestamp

        #Emito el log
        self.logger.log(app="Modelo", func="startNextRecord", level=0,
                        msg=f"Se inicia un nuevo registro con timestamp {timestamp}")
    

        self.idCurrentRecord += 1

    def getCurrentFolder(self):
        return self.currentFolder
    
    def setSide(self,side):
        if side=="r":
            self.side="R"

            self.logger.log(app="Model", func="setSide", level=0,
                            msg=f"Lado configurado a R")

        elif side=="l":
            self.side="L"

            self.logger.log(app="Model", func="setSide", level=0,
                            msg=f"Lado configurado a L")

        else:
            self.side="R"

            self.logger.log(app="Model", func="setSide", level=1,
                            msg=f"Lado configurado por defecto a R, ingreso {side} no válido")

