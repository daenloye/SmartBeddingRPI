use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvReading {
    pub temperature: f32,
    pub humidity: f32,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccelReading {
    pub gx: f32, pub gy: f32, pub gz: f32,
    pub ax: f32, pub ay: f32, pub az: f32,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureReading {
    pub matrix: [[u16; 12]; 16],
    pub timestamp: String,
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
    pub respiratory_rate: f32,
    pub heart_rate: f32,
    pub heart_rate_variability: f32,
}

#[derive(Serialize)]
pub struct SessionSchema {
    pub initTimestamp: String,
    pub finishTimestamp: String,
    pub dataRaw: crate::storage::DataRaw,
    pub dataProcessed: DataProcessed,
    pub measures: Measures,
    pub performance: Option<Performance>,
}