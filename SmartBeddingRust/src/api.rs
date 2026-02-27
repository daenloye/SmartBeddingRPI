use axum::{
    Router,
    routing::{get, post},
    Json,
    extract::{State, FromRequestParts},
    http::{StatusCode, Request, Method, header, request::Parts},
    response::IntoResponse,
    Extension,
};
use tower_http::cors::{Any, CorsLayer};
use std::sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}};
use crate::pressure::PressureMatrix;
use crate::acceleration::AccelerationModule;
use crate::config::CONFIG;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use chrono::Local;
use uuid::Uuid;
use async_trait::async_trait;
use std::process::Command;

// =============================
// STRUCTS
// =============================

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub result: bool,
    pub timestamp: String,
    pub data: Option<T>,
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct ApiLogin {
    pub code: String,
}

pub struct AppState {
    pub pressure: Arc<RwLock<PressureMatrix>>,
    pub accel: Arc<AccelerationModule>,
    pub session_token: RwLock<Option<String>>,
    pub mqtt_connected: Arc<AtomicBool>,
}

// =============================
// AUTH EXTRACTOR
// =============================

pub struct AuthUser;

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiResponse<()>>);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {

        let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();

        let state = parts
            .extensions
            .get::<Arc<AppState>>()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()> {
                    result: false,
                    timestamp: now.clone(),
                    data: None,
                    message: Some("State missing en Extensions".into()),
                }),
            ))?;

        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let session_token = state.session_token.read().unwrap();

        let is_valid = if let (Some(auth_str), Some(valid_token)) = (auth_header, &*session_token) {
            auth_str == format!("Bearer {}", valid_token)
        } else {
            false
        };

        if is_valid {
            Ok(AuthUser)
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<()> {
                    result: false,
                    timestamp: now,
                    data: None,
                    message: Some("Auth incorrecto o falta Token".into()),
                }),
            ))
        }
    }
}

// =============================
// START API
// =============================

pub async fn start_api(
    pressure: Arc<RwLock<PressureMatrix>>,
    accel: Arc<AccelerationModule>,
    mqtt_connected: Arc<AtomicBool>,
) {
    let shared_state = Arc::new(AppState {
        pressure,
        accel,
        session_token: RwLock::new(None),
        mqtt_connected,
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/auth", post(check_handler))
        .route("/verify", get(verify_handler))
        .route("/connectivity", get(connectivity_handler))
        .route("/storage", get(storage_handler))
        .route("/pressure", get(pressure_handler))
        .route("/accel", get(accel_handler))
        .layer(cors)
        .layer(Extension(shared_state.clone()))
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[API] Servidor iniciado en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// =============================
// HANDLERS
// =============================

async fn check_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ApiLogin>,
) -> Json<ApiResponse<String>> {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();

    if payload.code == CONFIG.api_code {
        let new_token = Uuid::new_v4().to_string();
        if let Ok(mut token_lock) = state.session_token.write() {
            *token_lock = Some(new_token.clone());
        }
        Json(ApiResponse {
            result: true,
            timestamp: now,
            data: Some(new_token),
            message: Some("Token generado".into()),
        })
    } else {
        Json(ApiResponse {
            result: false,
            timestamp: now,
            data: None,
            message: Some("Código incorrecto".into()),
        })
    }
}

async fn verify_handler(
    State(state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
) -> impl IntoResponse {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let auth_header = req.headers().get(header::AUTHORIZATION).and_then(|h| h.to_str().ok());
    let session_token = state.session_token.read().unwrap();

    let is_valid = if let (Some(auth_str), Some(valid_token)) = (auth_header, &*session_token) {
        auth_str == format!("Bearer {}", valid_token)
    } else {
        false
    };

    if is_valid {
        (StatusCode::OK, Json(ApiResponse::<()> { result: true, timestamp: now, data: None, message: Some("Auth correcto".into()) }))
    } else {
        (StatusCode::UNAUTHORIZED, Json(ApiResponse::<()> { result: false, timestamp: now, data: None, message: Some("Auth incorrecto".into()) }))
    }
}

async fn connectivity_handler(
    State(state): State<Arc<AppState>>,
    _user: AuthUser, 
) -> Json<ApiResponse<Value>> {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    
    // Obtener SSID actual
    let ssid = Command::new("iwgetid")
        .arg("-r")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "".into());

    // Escaneo de redes vía nmcli
    let networks = Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL", "dev", "wifi"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .filter_map(|l| {
                    let parts: Vec<&str> = l.split(':').collect();
                    if parts.len() >= 2 && !parts[0].is_empty() {
                        Some(json!({ "SSID": parts[0], "Strength": parts[1].parse::<i32>().unwrap_or(0) }))
                    } else { None }
                })
                .collect::<Vec<Value>>()
        })
        .unwrap_or_else(|_| vec![]);

    let data = json!({
        "APMode": ssid.is_empty(),
        "WifiSSID": if ssid.is_empty() { "Modo AP / No conectado" } else { &ssid },
        "BrokerMQTT": state.mqtt_connected.load(Ordering::SeqCst),
        "Networks": networks
    });

    Json(ApiResponse { result: true, timestamp: now, data: Some(data), message: None })
}

async fn storage_handler(
    _user: AuthUser, 
) -> Json<ApiResponse<Value>> {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let data = json!({
        "freeMb": 512,
        "totalMb": 10240,
        "registeredSessions": [
            {"date": now, "recordTime": 600, "sizeMb": 128}
        ]
    });
    Json(ApiResponse { result: true, timestamp: now, data: Some(data), message: None })
}

async fn pressure_handler(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> Json<ApiResponse<Value>> {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let current_matrix = if let Ok(s) = state.pressure.read() {
        json!(s.buffers[s.latest_idx])
    } else {
        json!(null)
    };
    Json(ApiResponse { result: true, timestamp: now, data: Some(current_matrix), message: None })
}

async fn accel_handler(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> Json<ApiResponse<Value>> {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let accel_data = state.accel.get_latest_data();
    Json(ApiResponse { result: true, timestamp: now, data: Some(json!(accel_data)), message: None })
}