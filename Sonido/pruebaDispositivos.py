import sounddevice as sd
import numpy as np
from scipy.io.wavfile import write
import subprocess


print(sd.query_devices())