use paho_mqtt as mqtt;
use std::time::Duration;
use crate::utils::logger;

const D_BROKER: &str = "tcp://192.168.1.100:1883";
const D_CLIENT_ID: &str = "SmartBedding_01";
const D_USER: &str = "admin";
const D_PASS: &str = "password123";

const TOPICS: &[&str] = &["smartbed/cmd", "smartbed/config"];
const QOS: &[i32] = &[1, 1];

pub struct MqttController{
    pub client: mqtt::Client,
}

impl MqttController{
    /// FASE 1: Constructor (Configuración estática)
    pub fn new() -> Self {
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(D_BROKER)
            .client_id(D_CLIENT_ID)
            .persistence(mqtt::PersistenceType::None) // Evita archivos temporales de persistencia
            .finalize();

        let client = mqtt::Client::new(create_opts).unwrap_or_else(|err| {
            logger("ERROR", &format!("MQTT Create Error: {:?}", err));
            panic!("Falló la creación del cliente MQTT");
        });

        Self { client }
    }

    /// FASE 2: Init (Preparación de parámetros de conexión)
    pub fn init(&self) {
        logger("MQTT", "Módulo inicializado y listo para conectar.");
    }

    /// FASE 3: Start (Conexión activa y escucha de hilos)
    pub fn start(&self) {
        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(20))
            .user_name(D_USER)
            .password(D_PASS)
            .clean_session(true)
            .automatic_reconnect(Duration::from_secs(1), Duration::from_secs(30))
            .finalize();

        // Iniciamos el receptor antes de conectar para no perder mensajes
        let rx = self.client.start_consuming();

        match self.client.connect(conn_opts) {
            Ok(_) => {
                logger("MQTT", &format!("Conexión establecida con {}", D_BROKER));
                
                // Suscripción a canales "quemados"
                if let Err(e) = self.client.subscribe_many(TOPICS, QOS) {
                    logger("ERROR", &format!("Error en suscripción inicial: {:?}", e));
                }
                
                // Disparamos el hilo de escucha (Background worker)
                self.spawn_message_handler(rx);
            }
            Err(e) => {
                logger("ERROR", &format!("Error al arrancar MQTT: {:?}", e));
            }
        }
    }

    /// Manejador interno de mensajes (Privado)
    fn spawn_message_handler(&self, rx: mqtt::Receiver<Option<mqtt::Message>>) {
        std::thread::spawn(move || {
            logger("MQTT", "Hilo de escucha de comandos activo.");
            for msg in rx {
                if let Some(msg) = msg {
                    // Aquí es donde procesarás los silvidos remotos o configuraciones
                    let payload = msg.payload_str(); 
                    logger("MQTT_IN", &format!("[{}] -> {}", msg.topic(), payload));
                }
            }
            logger("MQTT", "Hilo de escucha finalizado.");
        });
    }
}