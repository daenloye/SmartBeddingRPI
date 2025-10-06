
import numpy as np
from PyQt5.QtCore import QObject, QThread, QTimer, pyqtSignal
import os
import json

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

            # Aquí haces el procesamiento pesado (ej: guardar a disco)
            # save_to_disk(self.record)
            # o np.save(f"record_{self.record.initTimestamp}.npy", self.record.pressureData)

            #Genero la estructura de JSON de almacenamiento
            # self.logger.log(app="Modelo", func="RecordWorker", level=0,
            #                 msg=f"Estructura presión: {self.record.pressureData[0]}")
            # self.logger.log(app="Modelo", func="RecordWorker", level=0,
            #                 msg=f"Estructura aceleración: {self.record.acelerationData[0]}")

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
                        ]
                    },
                    "metrics": {
                        "respiratoryRate": None,
                        "heartRate": None,
                        "movementIndex": None,
                        "position": None
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
        self.initTimestamp=0
        self.finishTimestamp=0

    def storePressure(self, timestamp, pressure):
        self.pressureData.append({'timestamp': timestamp, 'measure': pressure})

    def storeAcceleration(self, timestamp, acceleration):
        self.acelerationData.append({'timestamp': timestamp, 'measure': acceleration})

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
        self.thread = None

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
        self.thread = QThread()
        worker = RecordWorker(self.currentRecord, self.currentFolder, self.idCurrentRecord, self.logger,self.debugFiles)
        worker.moveToThread(self.thread)

        self.thread.started.connect(worker.run)
        worker.finished.connect(self.thread.quit)
        worker.finished.connect(worker.deleteLater)
        self.thread.finished.connect(self.thread.deleteLater)

        self.thread.start()
        
        #Crear uno nuevo
        self.currentRecord = MinuteRecord()
        self.currentRecord.initTimestamp = timestamp

        #Emito el log
        self.logger.log(app="Modelo", func="startNextRecord", level=0,
                        msg=f"Se inicia un nuevo registro con timestamp {timestamp}")
    

        self.idCurrentRecord += 1

