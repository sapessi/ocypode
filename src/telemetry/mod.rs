pub(crate) mod collector;
pub(crate) mod producer;
pub(crate) mod scrub_analyzer;
pub(crate) mod short_shifting_analyzer;
pub(crate) mod slip_analyzer;
pub(crate) mod trailbrake_steering_analyzer;
pub(crate) mod wheelspin_analyzer;

use std::{
    collections::HashMap,
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

pub use collector::collect_telemetry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TelemetryAnnotation {
    Slip {
        prev_speed: f32,
        cur_speed: f32,
        is_slip: bool,
    },
    Scrub {
        avg_yaw_rate_change: f32,
        cur_yaw_rate_change: f32,
        is_scrubbing: bool,
    },
    ShortShifting {
        gear_change_rpm: f32,
        optimal_rpm: f32,
        is_short_shifting: bool,
    },
    TrailbrakeSteering {
        cur_trailbrake_steering: f32,
        is_excessive_trailbrake_steering: bool,
    },
    Wheelspin {
        avg_rpm_increase_per_gear: HashMap<u32, f32>,
        cur_gear: u32,
        cur_rpm_increase: f32,
        is_wheelspin: bool,
    },
}

impl Display for TelemetryAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryAnnotation::Slip {
                prev_speed: _,
                cur_speed: _,
                is_slip: _,
            } => write!(f, "slip"),
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: _,
                cur_yaw_rate_change: _,
                is_scrubbing: _,
            } => write!(f, "scrub"),
            TelemetryAnnotation::ShortShifting {
                gear_change_rpm: _,
                optimal_rpm: _,
                is_short_shifting: _,
            } => write!(f, "short_shift"),
            TelemetryAnnotation::TrailbrakeSteering {
                cur_trailbrake_steering: _,
                is_excessive_trailbrake_steering: _,
            } => write!(f, "trailbrake"),
            TelemetryAnnotation::Wheelspin {
                avg_rpm_increase_per_gear: _,
                cur_gear: _,
                cur_rpm_increase: _,
                is_wheelspin: _,
            } => write!(f, "wheelspin"),
        }
    }
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
pub enum TelemetryOutput {
    DataPoint(TelemetryPoint),
    SessionChange(SessionInfo),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryPoint {
    pub point_no: usize,
    pub point_epoch: u128,
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
    // steering angle as a % of max steering
    pub steering_pct: f32,
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

    pub annotations: Vec<TelemetryAnnotation>,
}

impl Default for TelemetryPoint {
    fn default() -> Self {
        Self {
            point_no: 0,
            point_epoch: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
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
            steering_pct: 0.,
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
            annotations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub track_name: String,
    pub track_configuration: String,
    pub max_steering_angle: f32,
    pub track_length: String,
    pub we_series_id: i32,
    pub we_session_id: i32,
    pub we_season_id: i32,
    pub we_sub_session_id: i32,
    pub we_league_id: i32,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            track_name: "Unknown".to_string(),
            track_configuration: "Unknown".to_string(),
            max_steering_angle: 0.,
            track_length: "".to_string(),
            we_series_id: 0,
            we_session_id: 0,
            we_season_id: 0,
            we_sub_session_id: 0,
            we_league_id: 0,
        }
    }
}

pub trait TelemetryAnalyzer {
    fn analyze(
        &mut self,
        telemetry_point: &TelemetryPoint,
        session_info: &SessionInfo,
    ) -> Vec<TelemetryAnnotation>;
}
