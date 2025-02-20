mod collector;
pub(crate) mod producer;
mod trailbrake_steering_analyzer;
mod wheelspin_analyzer;

use std::collections::HashMap;

pub use collector::collect_telemetry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

impl Default for TelemetryPoint {
    fn default() -> Self {
        Self {
            point_no: 0,
            lap_dist_pct: 0.,
            last_lap_time_s: 0.,
            best_lap_time_s: 0.,
            car_shift_ideal_rpm: 0.,
            cur_gear: 0,
            cur_rpm: 0.,
            cur_speed: 0.,
            lap_no: 0,
            throttle: 0.,
            brake: 0.,
            steering: 0.,
            abs_active: false,
            annotations: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub track_name: String,
    pub max_steering_angle: f32,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
        }
    }
}

pub trait TelemetryAnalyzer {
    fn analyze(
        &mut self,
        telemetry_point: &TelemetryPoint,
        session_info: &SessionInfo,
    ) -> HashMap<String, TelemetryAnnotation>;
}
