import datetime
import time
import queue
import threading
import numpy as np
import sounddevice as sd
from scipy.io.wavfile import write
from PyQt5.QtCore import QThread, pyqtSignal


class AudioRecorder(QThread):
    new_file = pyqtSignal(str)
    new_chunk = pyqtSignal(np.ndarray, str)
    error = pyqtSignal(str)

    def __init__(self, logger, folder, duration=60, samplerate=44100,
                 channels=1, chunk_interval=1.0, parent=None):
        super().__init__(parent)
        self.logger = logger
        self.folder = folder
        self.duration = duration
        self.samplerate = samplerate
        self.channels = channels
        self.chunk_interval = chunk_interval

        self._running = True
        self.numRecord = 1

        self.audio_q = queue.Queue()
        self.save_q = queue.Queue()

        self.buffer = []
        self.chunk_size = 4096
        self.samples_since_last_chunk = 0
        self.total_collected = 0

        self.saver_thread = threading.Thread(target=self._saver_loop, daemon=True)
        self.saver_thread.start()

    def callback(self, indata, frames, time_info, status):
        if status:
            self.logger.log(app="AudioRecorder", func="callback", level=1, msg=f"Status: {status}")
        self.audio_q.put(indata.copy())

    def _saver_loop(self):
        while self._running or not self.save_q.empty():
            try:
                filename, data = self.save_q.get(timeout=1)
                write(filename, self.samplerate, data)
                self.logger.log(app="AudioRecorder", func="_saver_loop", level=0,
                                msg=f"Guardado archivo {filename}")
                self.new_file.emit(filename)
            except queue.Empty:
                continue
            except Exception as e:
                self.logger.log(app="AudioRecorder", func="_saver_loop", level=2,
                                msg="Fallo al guardar archivo", error=e)
                self.error.emit(str(e))

    def stop(self):
        self._running = False

    def run(self):
        try:
            with sd.InputStream(
                samplerate=self.samplerate,
                channels=self.channels,
                dtype='int16',
                blocksize=self.chunk_size,
                callback=self.callback
            ):
                self.logger.log(app="AudioRecorder", func="run", level=0,
                                msg="Stream de audio continuo iniciado")

                samples_per_chunk = int(self.chunk_interval * self.samplerate)
                total_target_samples = int(self.duration * self.samplerate)

                while self._running:
                    try:
                        data = self.audio_q.get(timeout=0.1)
                        self.buffer.append(data)
                        frames = len(data)
                        self.samples_since_last_chunk += frames
                        self.total_collected += frames

                        # Emitir chunk exacto
                        while self.samples_since_last_chunk >= samples_per_chunk:
                            recording = np.concatenate(self.buffer, axis=0)
                            chunk_part = recording[:samples_per_chunk]
                            remainder = recording[samples_per_chunk:]
                            self.buffer = [remainder] if len(remainder) > 0 else []
                            self.samples_since_last_chunk -= samples_per_chunk

                            timestamp = datetime.datetime.now().strftime('%Y%m%d_%H%M%S_%f')[:-3]
                            self.new_chunk.emit(chunk_part, timestamp)
                            self.logger.log(app="AudioRecorder", func="run", level=0,
                                            msg=f"Chunk exacto emitido en {timestamp}")

                        # Guardar al alcanzar la duración total
                        if self.total_collected >= total_target_samples:
                            recording = np.concatenate(self.buffer, axis=0)
                            save_part = recording[:total_target_samples]
                            remainder = recording[total_target_samples:]
                            self.buffer = [remainder] if len(remainder) > 0 else []

                            datet = datetime.datetime.now().strftime('%Y%m%d_%H%M%S')
                            filename = f"{self.folder}/rec__{self.numRecord}_{datet}.wav"
                            self.save_q.put((filename, save_part))

                            self.logger.log(app="AudioRecorder", func="run", level=0,
                                            msg=f"Grabación #{self.numRecord} completada ({self.duration}s)")
                            self.numRecord += 1
                            self.total_collected = 0

                    except queue.Empty:
                        continue

                self.logger.log(app="AudioRecorder", func="run", level=0, msg="Stream detenido")

            # Espera para vaciar cola
            start_wait = time.time()
            while not self.save_q.empty() and time.time() - start_wait < 10:
                time.sleep(0.1)

        except Exception as e:
            self.logger.log(app="AudioRecorder", func="run", level=2,
                            msg="Error en stream de audio", error=e)
            self.error.emit(str(e))
