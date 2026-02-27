use axum::{
    Router, routing::get, Json, extract::State,
    http::{StatusCode, Request, Method, header},
    middleware::{self, Next},
    response::Response,
};
use tower_http::cors::{Any, CorsLayer}; // Necesitas: tower-http = { version = "0.5", features = ["cors"] }
use std::sync::{Arc, RwLock};
use crate::pressure::PressureMatrix;
use crate::acceleration::AccelerationModule;
use crate::config::CONFIG; // Usaremos el token de aquí
use serde::Serialize;
use chrono::Local;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub result: bool,
    pub timestamp: String,
    pub data: T,
}

pub struct AppState {
    pub pressure: Arc<RwLock<PressureMatrix>>,
    pub accel: Arc<AccelerationModule>,
}

pub async fn start_api(pressure: Arc<RwLock<PressureMatrix>>, accel: Arc<AccelerationModule>) {
    let shared_state = Arc::new(AppState { pressure, accel });

    // Configuración de CORS para que Angular no llore
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any) // En producción podrías poner solo la IP de la Pi
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/ping", get(ping_handler))
        // Aquí puedes meter más rutas protegidas
        .layer(middleware::from_fn(auth_middleware)) // Middleware de seguridad
        .layer(cors) // Capa de CORS
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[API] Servidor iniciado en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// MIDDLEWARE: Verifica el token estático
async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    // Comparamos con el token definido en tu CONFIG
    if let Some(auth_str) = auth_header {
        if auth_str == format!("Bearer {}", CONFIG.api_token) {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn ping_handler() -> Json<ApiResponse<Option<String>>> {
    Json(ApiResponse {
        result: true,
        timestamp: Local::now().format("%y/%m/%d %H:%M:%S%.3f").to_string(),
        data: None,
    })
}