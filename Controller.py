import sys
import asyncio
import signal

from PyQt5.QtCore import QCoreApplication
from qasync import QEventLoop

from LoggerManager import Logger


class Controlador:
    def __init__(self):
        # -----------------------------------------
        # Aplicación Qt (sin GUI, solo loop de eventos)
        # -----------------------------------------
        self.app = QCoreApplication(sys.argv)

        # -----------------------------------------
        # Loop de asyncio integrado con Qt
        # -----------------------------------------
        self.loop = QEventLoop(self.app)
        asyncio.set_event_loop(self.loop)

        # -----------------------------------------
        # Logger
        # -----------------------------------------
        self.logger = Logger()

    async def quit(self, sig=None):
        """
        Finaliza el bucle de eventos de forma segura
        """
        self.logger.log(app="Controlador", func="quit", level=0,
                        msg=f"Ha llegado la señal de salida ({sig})")
        self.loop.stop()

    def run(self):
        # Registrar manejadores de señales (Linux/Mac, no Windows)
        try:
            for sig in (signal.SIGINT, signal.SIGTERM):
                self.loop.add_signal_handler(sig, lambda s=sig: self.loop.create_task(self.quit(s)))
        except NotImplementedError:
            # En Windows no está implementado
            pass

        # Ejecutar el bucle de eventos
        with self.loop:
            try:
                self.loop.run_forever()
            except Exception as e:
                self.logger.log(app="Controlador", func="run", level=2,
                                msg="Error inesperado", error=e)
            finally:
                self.logger.log(app="Controlador", func="Close", level=0,
                                msg="Cerrando sistema")
                self.loop.close()


if __name__ == "__main__":
    c = Controlador()
    c.run()
