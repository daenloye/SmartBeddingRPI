import sounddevice as sd
from scipy.io.wavfile import write
from PyQt5.QtCore import QThread, pyqtSignal
import numpy as np
import datetime


class AudioRecorder(QThread):
    finished = pyqtSignal(str)

    def __init__(self, duration=59, samplerate=44100, channels=1, parent=None):
        super().__init__(parent)
        self.duration = duration
        self.samplerate = samplerate
        self.channels = channels
        self.filename = f"grabacion_{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.wav"

    def run(self):
        try:
            print(f"[INFO] Grabando por {self.duration} segundos...")
            recording = sd.rec(
                int(self.duration * self.samplerate),
                samplerate=self.samplerate,
                channels=self.channels,
                dtype='int16'
            )
            sd.wait()
            write(self.filename, self.samplerate, recording)
            print(f"[OK] Grabación guardada como {self.filename}")
            self.finished.emit(self.filename)
        except Exception as e:
            print(f"[ERROR] Falló la grabación: {e}")


# --- Ejemplo de uso directo (sin interfaz) ---
if __name__ == "__main__":
    from PyQt5.QtWidgets import QApplication
    import sys

    app = QApplication(sys.argv)

    worker = AudioRecorder(duration=59)
    worker.finished.connect(lambda f: print(f"[SIGNAL] Terminado: {f}"))
    worker.start()

    app.exec_()
