use axum::{
    Router, 
    routing::{get, post}, // Añadido post aquí
    Json, 
    extract::State,
    http::{StatusCode, Request, Method, header},
    middleware::{self, Next},
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

// Estructura Única y Global de Respuesta
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
    pub session_token: RwLock<Option<String>>, // CAMPO AÑADIDO AQUÍ
}

pub async fn start_api(pressure: Arc<RwLock<PressureMatrix>>, accel: Arc<AccelerationModule>) {
    let shared_state = Arc::new(AppState { 
        pressure, 
        accel,
        session_token: RwLock::new(None) // Inicializado correctamente
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any) 
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // RUTAS PÚBLICAS (No pasan por el middleware)
    let app = Router::new()
        .route("/check", post(check_handler)) // POST para validar código y dar token
        .layer(cors)
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[API] Servidor iniciado en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// HANDLER: /check (PÚBLICO)
// Aquí es donde Angular envía el código de CONFIG y recibe el UUID
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
            message: Some("Token generado exitosamente".to_string()),
        })
    } else {
        Json(ApiResponse {
            result: false,
            timestamp: now,
            data: None,
            message: Some("Código de acceso incorrecto".to_string()),
        })
    }
}

// MIDDLEWARE: Validación del Token Dinámico
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, Response> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let session_token = state.session_token.read().unwrap();

    if let (Some(auth_str), Some(valid_token)) = (auth_header, &*session_token) {
        if auth_str == format!("Bearer {}", valid_token) {
            return Ok(next.run(req).await);
        }
    }

    let error_response: ApiResponse<()> = ApiResponse {
        result: false,
        timestamp: Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string(),
        data: None,
        message: Some("No tienes autorización para esto".to_string()),
    };

    Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response())
}