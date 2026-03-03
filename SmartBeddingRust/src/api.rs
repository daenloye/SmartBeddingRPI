use axum::{
    Router,
    routing::{get, post, delete},
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
use tokio::task;

use std::path::Path;
use walkdir::{WalkDir, DirEntry};
use sysinfo::{Disks, Disk};

use std::collections::HashMap;

use std::fs; // <--- Fundamental

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

// AÑADE ESTO PARA SOLUCIONAR E0599
impl<T> ApiResponse<T> {
    pub fn new(result: bool, data: Option<T>, message: Option<String>) -> Self {
        Self {
            result,
            timestamp: Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string(),
            data,
            message,
        }
    }
}

#[derive(Deserialize)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct ApiLogin {
    pub code: String,
}

#[derive(Serialize)]
struct FileInfo {
    name: String,
    #[serde(rename = "sizeMb")]
    size_mb: f64,
    created: String,
}

#[derive(Serialize)]
struct RegisterGroup {
    created: String,
    name: String,
    path: String,
    #[serde(rename = "jsonFiles")]
    json_files: Vec<FileInfo>,
    #[serde(rename = "audioFiles")]
    audio_files: Vec<FileInfo>,
    #[serde(rename = "jsonUsedMb")]
    json_used_mb: f64,
    #[serde(rename = "audioUsedMb")]
    audio_used_mb: f64,
    #[serde(rename = "totalUsedMb")]
    total_used_mb: f64,
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
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_origin(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/auth", post(check_handler))
        .route("/verify", get(verify_handler))
        .route("/connectivity", get(connectivity_handler).post(wifi_connect_handler))
        .route("/storage", get(storage_handler).delete(storage_delete_handler))
        // .route("/pressure", get(pressure_handler))
        // .route("/accel", get(accel_handler))
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

pub async fn wifi_connect_handler(
    Json(payload): Json<WifiCredentials>,
) -> impl IntoResponse {
    // 1. Validaciones de seguridad básicas
    if payload.ssid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::new(false, None, Some("SSID no puede estar vacío".into())))
        ).into_response();
    }

    if let Some(ref pwd) = payload.password {
        if !pwd.is_empty() && pwd.len() < 8 {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::new(
                false, None, Some("La contraseña WPA requiere al menos 8 caracteres".into())
            ))).into_response();
        }
    }

    // 2. Verificar existencia de la red antes de intentar
    let scan_output = Command::new("nmcli")
        .args(["-t", "-f", "SSID", "dev", "wifi", "list", "--rescan", "yes"])
        .output();

    let exists = match scan_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| line.trim() == payload.ssid)
        }
        Err(_) => false,
    };

    if !exists {
        return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::new(
            false, None, Some(format!("La red '{}' no fue encontrada.", payload.ssid))
        ))).into_response();
    }

    // 3. Ejecución en Background con lógica de recuperación
    let ssid = payload.ssid.clone();
    let password = payload.password.clone();

    tokio::task::spawn(async move {
        // Espera para que el cliente reciba el OK
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;

        println!("[WIFI] Intentando conectar a {} (timeout 10s)...", ssid);

        let mut cmd = Command::new("nmcli");
        // --wait 10 es clave para que el comando falle si no obtiene IP rápido
        cmd.args(["--wait", "10", "device", "wifi", "connect", &ssid]);
        
        if let Some(pwd) = password {
            if !pwd.is_empty() { cmd.arg("password").arg(pwd); }
        }

        match cmd.output() {
            Ok(out) if out.status.success() => {
                println!("[WIFI] Conexión exitosa a {}", ssid);
            }
            _ => {
                println!("[WIFI] Error al conectar. Ejecutando rollback...");
                // Rollback: Desconectar el intento fallido y dejar que NetworkManager 
                // re-conecte a la red conocida con mejor señal
                let _ = Command::new("nmcli").args(["device", "disconnect", "wlan0"]).output();
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let _ = Command::new("nmcli").args(["device", "up", "wlan0"]).output();
            }
        }
    });

    // Respuesta inmediata con 200 OK
    (StatusCode::OK, Json(ApiResponse::<()>::new(
        true, None, 
        Some(format!("Intentando conectar a '{}'. Si falla, SmartBedding volverá a la red actual.", payload.ssid))
    ))).into_response()
}

async fn storage_handler(_user: AuthUser) -> Json<ApiResponse<Value>> {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    
    // 1. Espacio en disco (S.O.)
    let mut disks = Disks::new_with_refreshed_list();
    let (total_mb, free_mb) = disks.iter()
        .find(|disk: &&Disk| Path::new(CONFIG.storage_path).starts_with(disk.mount_point()))
        .map(|disk: &Disk| (
            disk.total_space() / 1024 / 1024, 
            disk.available_space() / 1024 / 1024
        ))
        .unwrap_or((0, 0));

    // 2. Escaneo y Agrupación por Carpeta
    // Usamos un HashMap donde la llave es el nombre de la carpeta
    let mut groups: HashMap<String, RegisterGroup> = HashMap::new();

    // WalkDir configurado para encontrar archivos en subcarpetas
    for entry in WalkDir::new(CONFIG.storage_path)
        .min_depth(2) // Empezamos en los archivos dentro de las carpetas de sesión
        .max_depth(3) 
    {
        if let Ok(e) = entry {
            let path = e.path();
            if path.is_file() {
                // Obtenemos el nombre de la carpeta padre (la sesión)
                if let Some(parent) = path.parent() {
                    let folder_name = parent.file_name().unwrap_or_default().to_string_lossy().to_string();
                    
                    // Si el grupo no existe, lo creamos
                    let group = groups.entry(folder_name.clone()).or_insert_with(|| {
                        let metadata = parent.metadata().ok();
                        RegisterGroup {
                            name: folder_name,
                            path: parent.to_string_lossy().to_string(),
                            created: metadata.and_then(|m| m.created().ok())
                                .map(|t| chrono::DateTime::<Local>::from(t).format("%Y/%m/%d %H:%M").to_string())
                                .unwrap_or_else(|| "---".into()),
                            json_files: Vec::new(),
                            audio_files: Vec::new(),
                            json_used_mb: 0.0,
                            audio_used_mb: 0.0,
                            total_used_mb: 0.0,
                        }
                    });

                    let metadata = e.metadata().ok();

                    let size_mb = metadata.as_ref().map(|m| m.len() as f64 / 1024.0 / 1024.0).unwrap_or(0.0);

                    let file_created = metadata
                                .and_then(|m| m.created().ok())
                                .map(|t| chrono::DateTime::<Local>::from(t).format("%Y/%m/%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "---".into());

                    let file_info = FileInfo {
                        name: e.file_name().to_string_lossy().to_string(),
                        size_mb: (size_mb * 100.0).round() / 100.0, // Redondear a 2 decimales
                        created: file_created,
                    };

                    match path.extension().and_then(|s| s.to_str()) {
                        Some("json") => {
                            group.json_used_mb += file_info.size_mb;
                            group.json_files.push(file_info);
                        },
                        Some("opus") => {
                            group.audio_used_mb += file_info.size_mb;
                            group.audio_files.push(file_info);
                        },
                        _ => {}
                    }
                    group.total_used_mb = group.json_used_mb + group.audio_used_mb;
                }
            }
        }
    }

    // Convertimos el HashMap a un Vec ordenado por nombre (opcional)
    let mut registers: Vec<RegisterGroup> = groups.into_values().collect();
    registers.sort_by(|a, b| b.name.cmp(&a.name)); // De más reciente a más viejo si usas nombres con fecha

    let data = json!({
        "registers": registers,
        "system": {
            "diskFreeMb": free_mb,
            "diskTotalMb": total_mb,
            "storageLimitMb": CONFIG.storage_max_mb
        }
    });

    Json(ApiResponse { 
        result: true, 
        timestamp: now, 
        data: Some(data), 
        message: None 
    })
}

async fn storage_delete_handler(
    _user: AuthUser,
) -> impl IntoResponse {
    let now = Local::now().format("%Y/%m/%d %H:%M:%S%.3f").to_string();
    let base_path = Path::new(CONFIG.storage_path);

    // 1. Obtener entradas del directorio
    let read_result = fs::read_dir(base_path);
    
    if let Ok(entries) = read_result {
        // Forzamos el tipo std::fs::DirEntry para evitar conflictos con walkdir
        let mut dirs: Vec<std::fs::DirEntry> = entries
            .filter_map(|r| r.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        // Ordenamos por fecha de creación (de más nuevo a más viejo)
        dirs.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.created())
                .unwrap_or_else(|_| std::time::SystemTime::now())
        });
        dirs.reverse(); 

        if dirs.len() <= 1 {
            return (StatusCode::OK, Json(ApiResponse::<()>{
                result: true,
                timestamp: now,
                data: None,
                message: Some("Nada que borrar, solo existe la sesión actual".into())
            }));
        }

        // 2. Borrar todas menos la primera (la más reciente)
        let mut deleted_count = 0;
        // Iteramos sobre los elementos a partir del segundo
        for entry in dirs.iter().skip(1) {
            let path_to_delete = entry.path();
            if fs::remove_dir_all(&path_to_delete).is_ok() {
                deleted_count += 1;
            }
        }

        (StatusCode::OK, Json(ApiResponse::<()>{
            result: true,
            timestamp: now,
            data: None,
            message: Some(format!("Limpieza total: {} sesiones eliminadas", deleted_count))
        }))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<()>{
            result: false,
            timestamp: now,
            data: None,
            message: Some("No se pudo leer el directorio de almacenamiento".into())
        }))
    }
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