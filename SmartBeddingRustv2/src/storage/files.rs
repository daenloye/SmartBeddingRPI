use std::fs;
use std::path::PathBuf;
use chrono::Local;
use crate::interfaces::*;
use crate::utils::logger;

// Constantes de Filtros
const B_RRS: [f64; 7] = [4.975743576868226e-05, 0.0, -0.00014927230730604678, 0.0, 0.00014927230730604678, 0.0, -4.975743576868226e-05];
const A_RRS: [f64; 7] = [1.0, -5.830766569820652, 14.185404142052889, -18.43141872929975, 13.489689338789688, -5.2728999261646115, 0.8599919781204693];
const B_CRS: [f64; 9] = [0.0010739281487746567, 0.0, -0.004295712595098627, 0.0, 0.006443568892647941, 0.0, -0.004295712595098627, 0.0, 0.0010739281487746567];
const A_CRS: [f64; 9] = [1.0, -6.4557706152374905, 18.656818730243238, -31.516992353914958, 34.03663934201975, -24.062919294682047, 10.877684610556427, -2.8761856141583015, 0.34094015209888484];

pub struct FileHandler {
    pub session_path: PathBuf,
}

impl FileHandler {
    pub fn new() -> Self {
        let mut path = PathBuf::from("SmartBeddingData");
        let now = Local::now().format("%Y%m%d_%H%M%S").to_string();
        path = path.join(format!("session_{}", now));
        let _ = fs::create_dir_all(&path);
        logger("STORAGE", &format!("Sesión física iniciada en: {:?}", path));
        Self { session_path: path }
    }

    pub fn save_session_json(&self, schema: &SessionSchema) {
        let file_path = self.session_path.join(format!("data_{}.json", Local::now().format("%H%M%S")));
        if let Ok(file) = fs::File::create(&file_path) {
            let _ = serde_json::to_writer(file, schema);
            logger("FILES", &format!("JSON guardado: {:?}", file_path));
        }
    }

    // --- LÓGICA DSP TRASLADADA ---
    pub fn run_dsp(&self, raw: &DataRaw) -> (Vec<f32>, Vec<f32>, f32) {
        let gx: Vec<f32> = raw.acceleration.iter().map(|a| a.gx).collect();
        let gy: Vec<f32> = raw.acceleration.iter().map(|a| a.gy).collect();
        let gz: Vec<f32> = raw.acceleration.iter().map(|a| a.gz).collect();

        let rrs = self.process_signal(&gx, &gy, &gz, &B_RRS, &A_RRS, [0.7, 0.22, 0.0775]);
        let crs = self.process_signal(&gx, &gy, &gz, &B_CRS, &A_CRS, [0.54633, 0.31161, 0.15108]);
        let resp_rate = self.calculate_resp_rate(&rrs, 20.0);

        (rrs, crs, resp_rate)
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