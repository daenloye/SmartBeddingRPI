use serde::{Serialize, Deserialize};

#[derive(Serialize, Clone, Default)]
pub struct AudioMeasures {
    pub db_avg: f32,
    pub db_max: f32,
    pub db_min: f32,
    pub zcr: f32,
    pub crest_factor: f32,
    pub silence_percent: f32,
}

#[derive(Serialize, Default)]
pub struct DataRaw {
    pub acceleration: Vec<AccelReading>,
    pub pressure: Vec<PressureReading>,
    pub environment: Vec<EnvReading>,
}

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
    pub audio: AudioMeasures,
    pub respiratory_rate: i32,
    pub heart_rate: i32,
    pub heart_rate_variability: i32,
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