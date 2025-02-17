mod collector;
mod wheelspin_analyzer;

use std::collections::HashMap;

pub use collector::collect_telemetry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TelemetryAnnotation {
    String(String),
    Float(f32),
    Int(i32),
    Bool(bool),
    NumberMap(HashMap<u32, f32>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryPoint {
    pub point_no: usize,
    pub lap_dist_pct: f32,
    pub last_lap_time_s: f32,
    pub best_lap_time_s: f32,
    pub car_shift_ideal_rpm: f32,
    pub cur_gear: u32,
    pub cur_rpm: f32,
    pub cur_speed: f32,
    pub lap_no: u32,
    pub throttle: f32,
    pub brake: f32,
    pub steering: f32,
    pub abs_active: bool,
    pub annotations: HashMap<String, TelemetryAnnotation>,
}

pub trait TelemetryAnalyzer {
    fn analyze(&mut self, telemetry_point: &TelemetryPoint)
        -> HashMap<String, TelemetryAnnotation>;
}
