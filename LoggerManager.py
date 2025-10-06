import logging
import os
import sys
import platform
from datetime import datetime
from typing import Optional

class Logger:
    def __init__(self, name: str = "SmartBedding"):
        self.logger = logging.getLogger(name)
        self.logger.setLevel(logging.INFO)
        self.logger.propagate = False

        formatter = logging.Formatter(
            "%(asctime)s.%(msecs)03d %(levelname)-8s %(app)15s: %(func)-25s -> %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S"
        )

        # Carpeta de logs
        log_dir = os.path.join(os.getcwd(), "Logs")
        os.makedirs(log_dir, exist_ok=True)

        # Crear nombre único de archivo
        today = datetime.now().strftime("%Y-%m-%d")
        file_number = 1
        while True:
            log_filename = f"{name}-{today}-{file_number}.log"
            log_path = os.path.join(log_dir, log_filename)
            if not os.path.exists(log_path):
                break
            file_number += 1

        # Handler para archivo
        file_handler = logging.FileHandler(log_path, mode='a', encoding='utf-8')
        file_handler.setLevel(logging.INFO)
        file_handler.setFormatter(formatter)

        # Handler para consola
        console_handler = logging.StreamHandler(sys.stdout)
        console_handler.setLevel(logging.INFO)
        console_handler.setFormatter(formatter)

        # Agregar ambos handlers
        self.logger.addHandler(file_handler)
        self.logger.addHandler(console_handler)

        # Log inicial
        platform_name = platform.system().lower()
        self.log(app="Logger", func="Logger", level=0,
                 msg=f"Logger iniciado en {platform_name.upper()}")

    def log(self, *, app: str, func: str = "", level: int = 0, msg: str = "", error: Optional[Exception] = None):
        try:
            extra = {"func": func, "app": app}

            if level == 0:
                self.logger.info(msg, extra=extra)
            elif level == 1:
                self.logger.warning(msg, extra=extra)
            else:
                if error:
                    _, _, tb = sys.exc_info()
                    line_info = f"Line {tb.tb_lineno}" if tb else "Unknown line"
                    self.logger.error(f"{line_info}: ({error})", extra=extra)
                else:
                    self.logger.error(msg, extra=extra)
        except Exception as e:
            print("Error interno al registrar logs:", e)
