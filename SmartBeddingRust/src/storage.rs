use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::Serialize;
use crate::pressure::{COL_SIZE, ROW_SIZE};
use crate::config::CONFIG;
use sysinfo::System; 

// --- COEFICIENTES DE FILTROS (Traducidos de Python) ---
const B_RRS: [f64; 7] = [4.975743576868226e-05, 0.0, -0.00014927230730604678, 0.0, 0.00014927230730604678, 0.0, -4.975743576868226e-05];
const A_RRS: [f64; 7] = [1.0, -5.830766569820652, 14.185404142052889, -18.43141872929975, 13.489689338789688, -5.2728999261646115, 0.8599919781204693];

const B_CRS: [f64; 9] = [0.0010739281487746567, 0.0, -0.004295712595098627, 0.0, 0.006443568892647941, 0.0, -0.004295712595098627, 0.0, 0.0010739281487746567];
const A_CRS: [f64; 9] = [1.0, -6.4557706152374905, 18.656818730243238, -31.516992353914958, 34.03663934201975, -24.062919294682047, 10.877684610556427, -2.8761856141583015, 0.34094015209888484];

#[derive(Serialize, Clone, Default)]
pub struct AudioMetrics {
    pub db_avg: f32,
    pub db_max: f32,
    pub db_min: f32,
    pub zcr: f32,
    pub crest_factor: f32,
    pub silence_percent: f32,
}

#[derive(Serialize, Clone)]
pub struct PressureSample {
    pub timestamp: String,
    pub measure: Arc<[[u16; COL_SIZE]; ROW_SIZE]>,
}

#[derive(Serialize, Clone)]
pub struct AccelSample {
    pub timestamp: String,
    pub measure: [f32; 6],
}

#[derive(Serialize, Clone)]
pub struct EnvironmentSample {
    pub timestamp: String,
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Serialize, Clone, Default)]
pub struct DataRaw {
    pub pressure: Vec<PressureSample>,
    pub acceleration: Vec<AccelSample>,
    pub environment: Vec<EnvironmentSample>,
}

#[derive(Serialize, Clone, Default)]
pub struct DataProcessed {
    pub rrs: Vec<f32>,
    pub crs: Vec<f32>,
}

#[derive(Serialize, Clone, Default)]
pub struct Performance {
    pub cpu_percent: f32,
    pub mem_percent: f32,
}

#[derive(Serialize, Clone, Default)]
pub struct Measures {
    pub audio: Option<AudioMetrics>,
    pub respiratory_rate: f32,
    pub heart_rate: f32,
    pub heart_rate_variability: f32,
}

#[derive(Serialize)]
pub struct SessionSchema {
    pub initTimestamp: String,
    pub finishTimestamp: String,
    pub dataRaw: DataRaw,
    pub dataProcessed: DataProcessed,
    pub measures: Measures,
    pub performance: Option<Performance>,
}

pub struct Storage;

impl Storage {
    pub fn init_path() -> PathBuf {
        let base_path = Path::new(CONFIG.storage_path);
        if !base_path.exists() {
            fs::create_dir_all(base_path).ok();
        }

        let mut max_idx = 0;
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(s) = name.strip_prefix("register_") {
                        if let Ok(n) = s.parse::<u32>() { if n > max_idx { max_idx = n; } }
                    }
                }
            }
        }

        let new_path = base_path.join(format!("register_{}", max_idx + 1));
        fs::create_dir_all(&new_path).expect("Error creando carpeta");
        new_path
    }

    fn apply_iir(data: &[f32], b: &[f64], a: &[f64]) -> Vec<f32> {
        let mut out = vec![0.0; data.len()];
        let order = b.len();
        if data.len() < order { return out; }

        for n in order..data.len() {
            let mut acc = 0.0;
            for i in 0..order {
                acc += b[i] * data[n - i] as f64;
                if i > 0 {
                    acc -= a[i] * out[n - i] as f64;
                }
            }
            out[n] = (acc / a[0]) as f32;
        }
        out
    }

    fn calculate_resp_rate(signal: &[f32], fs: f32) -> f32 {
        let mut crosses = 0;
        for i in 1..signal.len() {
            if (signal[i-1] <= 0.0 && signal[i] > 0.0) || (signal[i-1] >= 0.0 && signal[i] < 0.0) {
                crosses += 1;
            }
        }
        let duration_secs = signal.len() as f32 / fs;
        if duration_secs <= 0.0 { return 0.0; }
        (crosses as f32 / 2.0) * (60.0 / duration_secs)
    }

    pub fn save_session(mut session: SessionSchema, path: PathBuf, sys: &mut System) {
        // 1. Extraer ejes de aceleración
        let gx: Vec<f32> = session.dataRaw.acceleration.iter().map(|a| a.measure[0]).collect();
        let gy: Vec<f32> = session.dataRaw.acceleration.iter().map(|a| a.measure[1]).collect();
        let gz: Vec<f32> = session.dataRaw.acceleration.iter().map(|a| a.measure[2]).collect();

        // 2. Procesar RRS (Respiración)
        let gx_f_rrs = Self::apply_iir(&gx, &B_RRS, &A_RRS);
        let gy_f_rrs = Self::apply_iir(&gy, &B_RRS, &A_RRS);
        let gz_f_rrs = Self::apply_iir(&gz, &B_RRS, &A_RRS);

        session.dataProcessed.rrs = gx_f_rrs.iter().zip(gy_f_rrs.iter()).zip(gz_f_rrs.iter())
            .map(|((x, y), z)| 0.7 * x + 0.22 * y + 0.0775 * z)
            .collect();

        // 3. Procesar CRS (Cardíaco)
        let gx_f_crs = Self::apply_iir(&gx, &B_CRS, &A_CRS);
        let gy_f_crs = Self::apply_iir(&gy, &B_CRS, &A_CRS);
        let gz_f_crs = Self::apply_iir(&gz, &B_CRS, &A_CRS);

        session.dataProcessed.crs = gx_f_crs.iter().zip(gy_f_crs.iter()).zip(gz_f_crs.iter())
            .map(|((x, y), z)| 0.54633 * x + 0.31161 * y + 0.15108 * z)
            .collect();

        // 4. Calcular Medidas finales
        session.measures.respiratory_rate = Self::calculate_resp_rate(&session.dataProcessed.rrs, 20.0);
        // (Heart rate por implementar detección de picos)

        // 5. Performance del Sistema (sysinfo v0.30)
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        
        session.performance = Some(Performance {
            cpu_percent: sys.global_cpu_info().cpu_usage(),
            mem_percent: (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
        });

        // 6. Guardar archivo JSON
        if let Ok(file) = fs::File::create(path) {
            let _ = serde_json::to_writer_pretty(file, &session);
        }
    }
}