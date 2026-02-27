use axum::{
    Router, routing::get, Json, extract::State,
    http::{StatusCode, Request, Method, header},
    middleware::{self, Next},
    response::{Response, IntoResponse},
};
use tower_http::cors::{Any, CorsLayer};
use std::sync::{Arc, RwLock};
use crate::pressure::PressureMatrix;
use crate::acceleration::AccelerationModule;
use crate::config::CONFIG;
use serde::Serialize;
use chrono::Local;

// Estructura Única y Global de Respuesta
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub result: bool,
    pub timestamp: String,
    pub data: Option<T>,
    pub message: Option<String>,
}

pub struct AppState {
    pub pressure: Arc<RwLock<PressureMatrix>>,
    pub accel: Arc<AccelerationModule>,
}

pub async fn start_api(pressure: Arc<RwLock<PressureMatrix>>, accel: Arc<AccelerationModule>) {
    let shared_state = Arc::new(AppState { pressure, accel });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any) 
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/check", get(ping_handler))
        .layer(middleware::from_fn(auth_middleware))
        .layer(cors)
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[API] Servidor iniciado en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// MIDDLEWARE: Validación con el nuevo estándar de respuesta
async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, Response> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str == format!("Bearer {}", CONFIG.api_token) {
            return Ok(next.run(req).await);
        }
    }

    // Respuesta de error usando ApiResponse<T>
    let error_response: ApiResponse<()> = ApiResponse {
        result: false,
        timestamp: Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string(),
        data: None,
        message: Some("No tienes autorización para esto".to_string()),
    };

    Err((StatusCode::UNAUTHORIZED, Json(error_response)).into_response())
}

// HANDLER: Ping usando el nuevo estándar
async fn ping_handler() -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        result: true,
        timestamp: Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string(),
        data: Some("Servicio autorizado".to_string()),
        message: None,
    })
}