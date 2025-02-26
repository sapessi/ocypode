pub(crate) mod collector;
pub(crate) mod producer;
pub(crate) mod short_shifting_analyzer;
pub(crate) mod trailbrake_steering_analyzer;
pub(crate) mod wheelspin_analyzer;

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
pub struct TireInfo {
    left_carcass_temp: f32,
    middle_carcass_temp: f32,
    right_carcass_temp: f32,
    left_surface_temp: f32,
    middle_surface_temp: f32,
    right_surface_temp: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryPoint {
    pub point_no: usize,
    /// Meters traveled from S/F this lap
    pub lap_dist: f32,
    /// Percentage distance around lap
    pub lap_dist_pct: f32,
    /// Players last lap time
    pub last_lap_time_s: f32,
    /// Players best lap time
    pub best_lap_time_s: f32,
    /// Ideal RPM to shift gear
    pub car_shift_ideal_rpm: f32,
    /// Current gear
    pub cur_gear: u32,
    /// Current engine RPM
    pub cur_rpm: f32,
    /// Current speed
    pub cur_speed: f32,
    /// Lap number
    pub lap_no: u32,
    /// Throttle use. 0=off throttle to 1=full throttle
    pub throttle: f32,
    /// Brake use. Raw brake input 0=brake released to 1=max pedal force
    pub brake: f32,
    /// Steering wheel angle
    pub steering: f32,
    /// Whether ABS is currently active
    pub abs_active: bool,
    /// Latitude in decimal degrees
    pub lat: f32,
    /// Longitude in decimal degrees
    pub lon: f32,
    /// Lateral acceleration (including gravity), m/s^2
    pub lat_accel: f32,
    /// Longitudinal acceleration (including gravity), m/s^2
    pub lon_accel: f32,
    /// Pitch orientation (rad)
    pub pitch: f32,
    /// Pitch change rate (rad/s)
    pub pitch_rate: f32,
    /// Roll orientation (rad)
    pub roll: f32,
    /// Roll change rate (rad/s)
    pub roll_rate: f32,
    /// Yaw orientation (rad)
    pub yaw: f32,
    /// Yar change rate (rad/s)
    pub yaw_rate: f32,

    pub lf_tire_info: Option<TireInfo>,
    pub rf_tire_info: Option<TireInfo>,
    pub lr_tire_info: Option<TireInfo>,
    pub rr_tire_info: Option<TireInfo>,

    pub annotations: HashMap<String, TelemetryAnnotation>,
}

impl Default for TelemetryPoint {
    fn default() -> Self {
        Self {
            point_no: 0,
            lap_dist: 0.,
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
            lat: 0.,
            lon: 0.,
            lat_accel: 0.,
            lon_accel: 0.,
            pitch: 0.,
            pitch_rate: 0.,
            roll: 0.,
            roll_rate: 0.,
            yaw: 0.,
            yaw_rate: 0.,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
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
