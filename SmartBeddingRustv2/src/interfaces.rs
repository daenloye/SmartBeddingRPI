use serde::{Serialize, Deserialize};
use chrono::{DateTime, Local};

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