from PyQt5.QtCore import QObject, QThread, pyqtSignal, QTimer
import time

# -----------------------------
# Reader (fachada que maneja hilo y worker)
# -----------------------------
class ClockWorker(QObject):
    tick = pyqtSignal()

    def __init__(self, interval):
        super().__init__()
        self.interval = interval
        self.running = False

    def run(self):
        self.running = True
        next_time = time.perf_counter()

        while self.running:
            next_time += self.interval
            sleep_time = next_time - time.perf_counter()
            if sleep_time > 0:
                time.sleep(sleep_time)
            self.tick.emit()

    def stop(self):
        self.running = False