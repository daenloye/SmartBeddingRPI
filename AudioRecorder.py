import sounddevice as sd
from scipy.io.wavfile import write
from PyQt5.QtCore import QThread, pyqtSignal
import numpy as np
import datetime
import time


class AudioRecorder(QThread):
    new_file = pyqtSignal(str)   # se emite cada vez que se guarda un archivo
    error = pyqtSignal(str)      # se emite si ocurre un error

    def __init__(self, logger,folder,duration=60, samplerate=44100, channels=1, parent=None):
        super().__init__(parent)
        self.logger = logger
        self.folder = folder
        self.duration = duration
        self.samplerate = samplerate
        self.channels = channels
        self._running = True     # control de parada externa

        self.numRecord=1

    def stop(self):
        """Detiene la grabacion continua de forma segura."""
        self._running = False

    def run(self):
        while self._running:
            try:

                datet= datetime.datetime.now().strftime('%Y%m%d_%H%M%S')
                # Genera nombre único por fecha/hora
                filename = f"{self.folder}/rec__{self.numRecord}_{datet}.wav"

                if self.logger:
                    self.logger.log(app="AudioRecorder", func="start", level=0,
                                    msg=f"Iniciado grabacion de audio")

                # Graba
                recording = sd.rec(
                    int(self.duration * self.samplerate),
                    samplerate=self.samplerate,
                    channels=self.channels,
                    dtype='int16'
                )
                sd.wait()  # espera que termine la grabación

                # Guarda archivo
                write(filename, self.samplerate, recording)

                # Emite señal con el nombre del archivo
                self.new_file.emit(filename)

                datet2= datetime.datetime.now().strftime('%Y%m%d_%H%M%S')

                self.logger.log(app="AudioRecorder", func="start", level=0,msg=f"Se termina la grabacion de audio {datet2}")

                # Pausa corta (opcional)
                time.sleep(0.1)

                self.numRecord+=1

            except Exception as e:
                self.logger.log(app="AudioRecorder", func="start", level=2,msg="Fallo en la grabación", error=e)
                time.sleep(1)  # evita bucle rápido de errores

        #print("[INFO] Grabador continuo detenido.")


# --- Ejemplo de uso directo ---
# if __name__ == "__main__":
#     from PyQt5.QtWidgets import QApplication
#     import sys

#     app = QApplication(sys.argv)

#     recorder = AudioRecorder(duration=10)  # 10 s por ciclo para prueba
#     recorder.new_file.connect(lambda f: print(f"[SIGNAL] Nuevo archivo: {f}"))
#     recorder.error.connect(lambda e: print(f"[SIGNAL] Error: {e}"))

#     recorder.start()

#     # Detener manualmente luego de 3 ciclos de 10s (para probar)
#     from PyQt5.QtCore import QTimer
#     QTimer.singleShot(31000*10, recorder.stop)  # detiene a los 31 s

#     sys.exit(app.exec_())
