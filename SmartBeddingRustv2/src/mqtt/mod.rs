use paho_mqtt as mqtt;
use std::time::Duration;
use crate::utils::logger;
use tokio::time::sleep;
use serde::Deserialize; // Asegúrate de tener serde con feature "derive"
use std::sync::{Arc, RwLock};

const D_BROKER: &str = "tcp://3.90.24.183:8807";
const D_USER: &str = "smartbedding_publisher";
const D_PASS: &str = "Sb998?-Tx";
const D_CLIENT_ID: &str = "000001";

// Estructura para parsear la respuesta del servidor
#[derive(Deserialize, Debug)]
struct ServerResponse {
    date: String,      // El timestamp que viste: "1773953549"
    side: String,      // El lado: "l" o "r"
    error: Option<String>,
}

pub struct MqttController {
    pub client: mqtt::AsyncClient,
    pub bedding_id: String,
    // Estado compartido: (Offset de tiempo, Lado de la cama)
    pub state: Arc<RwLock<(i64, String)>>, 
}

impl MqttController {
    pub fn new() -> Self {
        let bedding_id = D_CLIENT_ID.to_string();
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(D_BROKER)
            .client_id(&bedding_id)
            .persistence(mqtt::PersistenceType::None)
            .finalize();

        let client = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|err| {
            logger("ERROR", &format!("MQTT Async Create Error: {:?}", err));
            panic!("Falló la creación del cliente MQTT");
        });

        Self { 
            client, 
            bedding_id,
            state: Arc::new(RwLock::new((0, "n/a".to_string()))),
        }
    }

    pub fn init(&self) {
        logger("MQTT", &format!("Módulo Async listo (ID: {})", self.bedding_id));
    }

    pub fn start(&self) {
        let mut client = self.client.clone();
        let bedding_id = self.bedding_id.clone();
        let state = Arc::clone(&self.state);
        let mut strm = client.get_stream(25);

        logger("MQTT", "Iniciando loop de eventos asíncrono...");

        tokio::spawn(async move {
            let conn_opts = mqtt::ConnectOptionsBuilder::new()
                .keep_alive_interval(Duration::from_secs(60))
                .user_name(D_USER)
                .password(D_PASS)
                .clean_session(true)
                .automatic_reconnect(Duration::from_secs(1), Duration::from_secs(30))
                .finalize();

            if let Err(e) = client.connect(conn_opts).await {
                logger("ERROR", &format!("Error conexión MQTT: {:?}", e));
            } else {
                logger("MQTT", "Conectado al broker de producción (Async)");
                let topic = format!("sb/response/{}", bedding_id);
                let _ = client.subscribe(&topic, 1).await;
                
                sleep(Duration::from_secs(1)).await;
                let init_topic = format!("sb/init/{}", bedding_id);
                let payload = format!(r#"{{"s": "{}"}}"#, bedding_id);
                let msg = mqtt::Message::new(init_topic, payload, 1);
                let _ = client.publish(msg).await;
            }

            // Loop de escucha de mensajes
            while let Ok(msg_opt) = strm.recv().await {
                if let Some(msg) = msg_opt {
                    let topic = msg.topic();
                    let payload = msg.payload_str();
                    
                    // Si es el mensaje de respuesta, calculamos el offset
                    if topic.contains("response") {
                        if let Ok(res) = serde_json::from_str::<ServerResponse>(&payload) {
                            if let Ok(srv_ts) = res.date.parse::<i64>() {
                                let local_ts = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap().as_secs() as i64;
                                
                                let offset = srv_ts - local_ts;
                                
                                // Guardamos en el estado compartido
                                if let Ok(mut w) = state.write() {
                                    *w = (offset, res.side.clone());
                                    logger("MQTT", &format!("Sincronizado: Offset {}s, Lado {}", offset, res.side));
                                }
                            }
                        }
                    }
                    logger("MQTT_IN", &format!("[{}] -> {}", topic, payload));
                }
            }
        });
    }

    /// Obtiene el timestamp corregido (solo para enviar datos)
    pub fn get_synced_timestamp(&self) -> i64 {
        let local_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap().as_secs() as i64;
        
        if let Ok(r) = self.state.read() {
            return local_now + r.0;
        }
        local_now
    }

    pub async fn publish_record(&self, payload: String) {
        if self.client.is_connected() {
            let topic = format!("sb/record/{}", self.bedding_id);
            let msg = mqtt::Message::new(topic, payload, 1);
            let _ = self.client.publish(msg).await;
        }
    }
}