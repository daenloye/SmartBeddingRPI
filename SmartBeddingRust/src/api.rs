use axum::{
    Router,
    routing::{get, post},
    Json,
    extract::State,
    http::{StatusCode, Request, Method, header},
    middleware::Next,
    response::{Response, IntoResponse},
};
use tower_http::cors::{Any, CorsLayer};
use std::sync::{Arc, RwLock};
use crate::pressure::PressureMatrix;
use crate::acceleration::AccelerationModule;
use crate::config::CONFIG;
use serde::{Serialize, Deserialize};
use chrono::Local;
use uuid::Uuid;

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
}

// =============================
// START API
// =============================

pub async fn start_api(
    pressure: Arc<RwLock<PressureMatrix>>,
    accel: Arc<AccelerationModule>,
) {
    let shared_state = Arc::new(AppState {
        pressure,
        accel,
        session_token: RwLock::new(None),
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/auth", post(check_handler))
        .route("/verify", get(verify_handler))
        .layer(cors)
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[API] Servidor iniciado en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// =============================
// AUTH LOGIN
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
            message: Some("Token generado".to_string()),
        })
    } else {
        Json(ApiResponse {
            result: false,
            timestamp: now,
            data: None,
            message: Some("Código incorrecto".to_string()),
        })
    }
}

// =============================
// VERIFY (RESPONDE SI / NO)
// =============================

async fn verify_handler(
    State(state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
) -> impl IntoResponse {

    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();

    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let session_token = state.session_token.read().unwrap();

    let is_valid = if let (Some(auth_str), Some(valid_token)) = (auth_header, &*session_token) {
        auth_str == format!("Bearer {}", valid_token)
    } else {
        false
    };

    if is_valid {
        (
            StatusCode::OK,
            Json(ApiResponse::<()> {
                result: true,
                timestamp: now,
                data: None,
                message: Some("Auth correcto".to_string()),
            })
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()> {
                result: false,
                timestamp: now,
                data: None,
                message: Some("Auth incorrecto".to_string()),
            })
        )
    }
}