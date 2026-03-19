use std::fs;
use std::path::PathBuf;
use chrono::Local;
use serde::Serialize;
use sysinfo::{System, SystemExt, CpuExt}; // Añade SystemExt y CpuExt
use crate::utils::logger;
use crate::interfaces::*;

const B_RRS: [f64; 7] = [4.975743576868226e-05, 0.0, -0.00014927230730604678, 0.0, 0.00014927230730604678, 0.0, -4.975743576868226e-05];
const A_RRS: [f64; 7] = [1.0, -5.830766569820652, 14.185404142052889, -18.43141872929975, 13.489689338789688, -5.2728999261646115, 0.8599919781204693];

const B_CRS: [f64; 9] = [0.0010739281487746567, 0.0, -0.004295712595098627, 0.0, 0.006443568892647941, 0.0, -0.004295712595098627, 0.0, 0.0010739281487746567];
const A_CRS: [f64; 9] = [1.0, -6.4557706152374905, 18.656818730243238, -31.516992353914958, 34.03663934201975, -24.062919294682047, 10.877684610556427, -2.8761856141583015, 0.34094015209888484];

#[derive(Serialize, Clone, Default)]
pub struct DataRaw {
    pub pressure: Vec<PressureReading>,
    pub acceleration: Vec<AccelReading>,
    pub environment: Vec<EnvReading>,
}

pub struct StorageController {
    pub session_path: PathBuf,
}

impl StorageController {
    pub fn new() -> Self {
        Self { session_path: PathBuf::from("SmartBeddingData") }
    }

    pub fn init(&mut self) {
        let now = Local::now().format("%Y%m%d_%H%M%S").to_string();
        self.session_path = self.session_path.join(format!("session_{}", now));
        let _ = fs::create_dir_all(&self.session_path);
        logger("STORAGE", &format!("Sesión iniciada en: {:?}", self.session_path));
    }

    pub fn process_and_save(&self, raw_data: DataRaw, start_time: String) {
        let finish_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let file_path = self.session_path.join(format!("data_{}.json", Local::now().format("%H%M%S")));

        // 1. DSP: Extraer ejes
        let gx: Vec<f32> = raw_data.acceleration.iter().map(|a| a.gx).collect();
        let gy: Vec<f32> = raw_data.acceleration.iter().map(|a| a.gy).collect();
        let gz: Vec<f32> = raw_data.acceleration.iter().map(|a| a.gz).collect();

        // 2. Filtrado e integración
        let rrs = self.process_signal(&gx, &gy, &gz, &B_RRS, &A_RRS, [0.7, 0.22, 0.0775]);
        let crs = self.process_signal(&gx, &gy, &gz, &B_CRS, &A_CRS, [0.54633, 0.31161, 0.15108]);
        let resp_rate = self.calculate_resp_rate(&rrs, 20.0);

        // 3. Sistema
        let mut sys = System::new_all();
        sys.refresh_all();
        let performance = Performance {
            cpu_percent: sys.global_cpu_info().cpu_usage(),
            mem_percent: (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
        };

        // 4. Schema final
        let schema = SessionSchema {
            initTimestamp: start_time,
            finishTimestamp: finish_time,
            dataRaw: raw_data, 
            dataProcessed: DataProcessed { rrs, crs },
            measures: Measures { respiratory_rate: resp_rate, ..Default::default() },
            performance: Some(performance),
        };

        // 5. Escritura
        if let Ok(file) = fs::File::create(&file_path) {
            let _ = serde_json::to_writer(file, &schema);
            logger("STORAGE", &format!("Minuto guardado en {:?}", file_path));
        }
    }

    fn apply_iir(&self, data: &[f32], b: &[f64], a: &[f64]) -> Vec<f32> {
        let mut out = vec![0.0; data.len()];
        let order = b.len();
        if data.len() < order { return out; }
        for n in order..data.len() {
            let mut acc = 0.0;
            for i in 0..order {
                acc += b[i] * data[n - i] as f64;
                if i > 0 { acc -= a[i] * out[n - i] as f64; }
            }
            out[n] = (acc / a[0]) as f32;
        }
        out
    }

    fn process_signal(&self, x: &[f32], y: &[f32], z: &[f32], b: &[f64], a: &[f64], w: [f32; 3]) -> Vec<f32> {
        let fx = self.apply_iir(x, b, a);
        let fy = self.apply_iir(y, b, a);
        let fz = self.apply_iir(z, b, a);
        fx.iter().zip(fy.iter()).zip(fz.iter())
            .map(|((vx, vy), vz)| w[0]*vx + w[1]*vy + w[2]*vz)
            .collect()
    }

    fn calculate_resp_rate(&self, signal: &[f32], fs: f32) -> f32 {
        let mut crosses = 0;
        for i in 1..signal.len() {
            if (signal[i-1] <= 0.0 && signal[i] > 0.0) || (signal[i-1] >= 0.0 && signal[i] < 0.0) { crosses += 1; }
        }
        let dur = signal.len() as f32 / fs;
        if dur <= 0.0 { 0.0 } else { (crosses as f32 / 2.0) * (60.0 / dur) }
    }
}