use axum::{Router, routing::get, Json, extract::State};
use std::sync::{Arc, RwLock};
use crate::pressure::PressureMatrix;
use crate::acceleration::AccelerationModule;
use serde::Serialize;

// Estado compartido que la API "verá"
pub struct AppState {
    pub pressure: Arc<RwLock<PressureMatrix>>,
    pub accel: Arc<AccelerationModule>,
}

#[derive(Serialize)]
pub struct LiveData {
    pub accel: [f32; 6],
    pub pressure_sum: u32, // Un ejemplo de dato rápido
}

pub async fn start_api(pressure: Arc<RwLock<PressureMatrix>>, accel: Arc<AccelerationModule>) {
    let shared_state = Arc::new(AppState { pressure, accel });

    let app = Router::new()
        .route("/live", get(get_live_data))
        .route("/pressure", get(get_full_pressure))
        .with_state(shared_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[API] Servidor listo en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handlers
async fn get_live_data(State(state): State<Arc<AppState>>) -> Json<LiveData> {
    let accel = state.accel.get_latest_data();
    let p_sum = if let Ok(p) = state.pressure.read() {
        p.buffers[p.latest_idx].iter().flatten().map(|&v| v as u32).sum()
    } else { 0 };

    Json(LiveData { accel, pressure_sum: p_sum })
}

async fn get_full_pressure(State(state): State<Arc<AppState>>) -> Json<Vec<Vec<u16>>> {
    let p = state.pressure.read().unwrap();
    let matrix = p.buffers[p.latest_idx].iter()
        .map(|row| row.to_vec())
        .collect();
    Json(matrix)
}