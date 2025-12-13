pub(crate) mod bottoming_out_analyzer;
pub(crate) mod brake_lock_analyzer;
pub(crate) mod collector;
pub(crate) mod entry_oversteer_analyzer;
pub(crate) mod mid_corner_analyzer;
pub(crate) mod producer;
pub(crate) mod scrub_analyzer;
pub(crate) mod short_shifting_analyzer;
pub(crate) mod slip_analyzer;
pub(crate) mod tire_temperature_analyzer;
pub(crate) mod trailbrake_steering_analyzer;
pub(crate) mod wheelspin_analyzer;

use std::{
    collections::HashMap,
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

pub use collector::collect_telemetry;

/// For ACC, estimate optimal shift point as a percentage of max RPM
/// Most cars benefit from shifting around 85-92% of max RPM for optimal power
const ACC_OPTIMAL_SHIFT_PCT: f32 = 0.92;
use serde::{Deserialize, Serialize};
use simetry::Moment;

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
    EntryOversteer {
        expected_yaw_rate: f32,
        actual_yaw_rate: f32,
        is_oversteer: bool,
    },
    MidCornerUndersteer {
        speed_loss: f32,
        is_understeer: bool,
    },
    MidCornerOversteer {
        yaw_rate_excess: f32,
        is_oversteer: bool,
    },
    FrontBrakeLock {
        abs_activation_count: usize,
        is_front_lock: bool,
    },
    RearBrakeLock {
        abs_activation_count: usize,
        is_rear_lock: bool,
    },
    TireOverheating {
        avg_temp: f32,
        optimal_max: f32,
        is_overheating: bool,
    },
    TireCold {
        avg_temp: f32,
        optimal_min: f32,
        is_cold: bool,
    },
    BottomingOut {
        pitch_change: f32,
        speed_loss: f32,
        is_bottoming: bool,
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
            TelemetryAnnotation::EntryOversteer {
                expected_yaw_rate: _,
                actual_yaw_rate: _,
                is_oversteer: _,
            } => write!(f, "entry_oversteer"),
            TelemetryAnnotation::MidCornerUndersteer {
                speed_loss: _,
                is_understeer: _,
            } => write!(f, "mid_corner_understeer"),
            TelemetryAnnotation::MidCornerOversteer {
                yaw_rate_excess: _,
                is_oversteer: _,
            } => write!(f, "mid_corner_oversteer"),
            TelemetryAnnotation::FrontBrakeLock {
                abs_activation_count: _,
                is_front_lock: _,
            } => write!(f, "front_brake_lock"),
            TelemetryAnnotation::RearBrakeLock {
                abs_activation_count: _,
                is_rear_lock: _,
            } => write!(f, "rear_brake_lock"),
            TelemetryAnnotation::TireOverheating {
                avg_temp: _,
                optimal_max: _,
                is_overheating: _,
            } => write!(f, "tire_overheating"),
            TelemetryAnnotation::TireCold {
                avg_temp: _,
                optimal_min: _,
                is_cold: _,
            } => write!(f, "tire_cold"),
            TelemetryAnnotation::BottomingOut {
                pitch_change: _,
                speed_loss: _,
                is_bottoming: _,
            } => write!(f, "bottoming_out"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TireInfo {
    pub left_carcass_temp: f32,
    pub middle_carcass_temp: f32,
    pub right_carcass_temp: f32,
    pub left_surface_temp: f32,
    pub middle_surface_temp: f32,
    pub right_surface_temp: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum GameSource {
    IRacing,
    ACC,
}

/// Intermediate telemetry representation that captures all possible telemetry data points
/// from supported racing simulations. This struct decouples analyzers from game-specific
/// implementations and eliminates the need for unsafe downcasting.
///
/// Fields use explicit unit suffixes for clarity:
/// - `_rad` for radians
/// - `_rps` for radians per second
/// - `_mps` for meters per second
/// - `_mps2` for meters per second squared
/// - `_deg` for degrees
/// - `_m` for meters
/// - `_s` for seconds
/// - `_pct` for percentage (0.0 to 1.0)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryData {
    // Metadata
    pub point_no: usize,
    pub timestamp_ms: u128,
    pub game_source: GameSource,

    // Vehicle state
    pub gear: Option<i8>,
    pub speed_mps: Option<f32>,
    pub engine_rpm: Option<f32>,
    pub max_engine_rpm: Option<f32>,
    pub shift_point_rpm: Option<f32>,

    // Inputs
    pub throttle: Option<f32>,
    pub brake: Option<f32>,
    pub clutch: Option<f32>,
    pub steering_angle_rad: Option<f32>,
    pub steering_pct: Option<f32>,

    // Position and lap data
    pub lap_distance_m: Option<f32>,
    pub lap_distance_pct: Option<f32>,
    pub lap_number: Option<u32>,

    // Timing
    pub last_lap_time_s: Option<f32>,
    pub best_lap_time_s: Option<f32>,

    // Flags and states
    pub is_pit_limiter_engaged: Option<bool>,
    pub is_in_pit_lane: Option<bool>,
    pub is_abs_active: Option<bool>,

    // GPS coordinates (iRacing only)
    pub latitude_deg: Option<f32>,
    pub longitude_deg: Option<f32>,

    // Acceleration
    pub lateral_accel_mps2: Option<f32>,
    pub longitudinal_accel_mps2: Option<f32>,

    // Orientation
    pub pitch_rad: Option<f32>,
    pub pitch_rate_rps: Option<f32>,
    pub roll_rad: Option<f32>,
    pub roll_rate_rps: Option<f32>,
    pub yaw_rad: Option<f32>,
    pub yaw_rate_rps: Option<f32>,

    // Tire data
    pub lf_tire_info: Option<TireInfo>,
    pub rf_tire_info: Option<TireInfo>,
    pub lr_tire_info: Option<TireInfo>,
    pub rr_tire_info: Option<TireInfo>,

    // Analyzer annotations
    pub annotations: Vec<TelemetryAnnotation>,
}

impl Default for TelemetryData {
    fn default() -> Self {
        Self {
            point_no: 0,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            game_source: GameSource::IRacing,
            gear: None,
            speed_mps: None,
            engine_rpm: None,
            max_engine_rpm: None,
            shift_point_rpm: None,
            throttle: None,
            brake: None,
            clutch: None,
            steering_angle_rad: None,
            steering_pct: None,
            lap_distance_m: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            is_abs_active: None,
            latitude_deg: None,
            longitude_deg: None,
            lateral_accel_mps2: None,
            longitudinal_accel_mps2: None,
            pitch_rad: None,
            pitch_rate_rps: None,
            roll_rad: None,
            roll_rate_rps: None,
            yaw_rad: None,
            yaw_rate_rps: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        }
    }
}

impl TelemetryData {
    /// Convert iRacing SimState to TelemetryData.
    ///
    /// Extracts all available telemetry fields from iRacing. Currently, simetry 0.2.3
    /// only exposes fields through the base Moment trait. iRacing-specific fields like
    /// steering angle, GPS coordinates, orientation, and tire temperatures are not
    /// accessible through the current simetry API.
    ///
    /// Fields extracted from Moment trait:
    /// - Vehicle state (gear, speed, RPM, shift point)
    /// - Inputs (throttle, brake, clutch)
    /// - Flags (pit limiter, pit lane)
    ///
    /// Fields not available (set to None):
    /// - Steering angle and percentage
    /// - Lap distance and position data
    /// - Lap times
    /// - ABS status
    /// - GPS coordinates
    /// - Acceleration data
    /// - Orientation (pitch, roll, yaw) and rates
    /// - Tire temperatures
    ///
    /// TODO: These fields require either:
    /// 1. Accessing iRacing's raw shared memory directly
    /// 2. Extending the simetry library to expose these fields
    /// 3. Using a different approach to access the telemetry data
    #[cfg(windows)]
    pub fn from_iracing_state(state: &simetry::iracing::SimState, point_no: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        use uom::si::angular_velocity::revolution_per_minute;
        use uom::si::velocity::meter_per_second;

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // Extract base fields from Moment trait
        let gear = state.vehicle_gear();
        let speed_mps = state
            .vehicle_velocity()
            .map(|v| v.get::<meter_per_second>() as f32);
        let engine_rpm = state
            .vehicle_engine_rotation_speed()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32);
        let max_engine_rpm = state
            .vehicle_max_engine_rotation_speed()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32);
        let shift_point_rpm = state
            .shift_point()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32);

        // Extract pedal inputs from Moment trait
        let pedals = state.pedals();
        let throttle = pedals.as_ref().map(|p| p.throttle as f32);
        let brake = pedals.as_ref().map(|p| p.brake as f32);
        let clutch = pedals.as_ref().map(|p| p.clutch as f32);

        // Extract flags from Moment trait
        let is_pit_limiter_engaged = state.is_pit_limiter_engaged();
        let is_in_pit_lane = state.is_vehicle_in_pit_lane();

        // iRacing-specific fields are not accessible through simetry 0.2.3
        // These would require direct access to iRacing's shared memory
        let steering_angle_rad = None;
        let steering_pct = None;
        let lap_distance_m = None;
        let lap_distance_pct = None;
        let lap_number = None;
        let last_lap_time_s = None;
        let best_lap_time_s = None;
        let is_abs_active = None;
        let latitude_deg = None;
        let longitude_deg = None;
        let lateral_accel_mps2 = None;
        let longitudinal_accel_mps2 = None;
        let pitch_rad = None;
        let pitch_rate_rps = None;
        let roll_rad = None;
        let roll_rate_rps = None;
        let yaw_rad = None;
        let yaw_rate_rps = None;
        let lf_tire_info = None;
        let rf_tire_info = None;
        let lr_tire_info = None;
        let rr_tire_info = None;

        Self {
            point_no,
            timestamp_ms,
            game_source: GameSource::IRacing,
            gear,
            speed_mps,
            engine_rpm,
            max_engine_rpm,
            shift_point_rpm,
            throttle,
            brake,
            clutch,
            steering_angle_rad,
            steering_pct,
            lap_distance_m,
            lap_distance_pct,
            lap_number,
            last_lap_time_s,
            best_lap_time_s,
            is_pit_limiter_engaged,
            is_in_pit_lane,
            is_abs_active,
            latitude_deg,
            longitude_deg,
            lateral_accel_mps2,
            longitudinal_accel_mps2,
            pitch_rad,
            pitch_rate_rps,
            roll_rad,
            roll_rate_rps,
            yaw_rad,
            yaw_rate_rps,
            lf_tire_info,
            rf_tire_info,
            lr_tire_info,
            rr_tire_info,
            annotations: Vec::new(),
        }
    }

    /// Convert ACC SimState to TelemetryData.
    ///
    /// Extracts all available telemetry fields from ACC. ACC provides access to most
    /// telemetry data through the physics and graphics structures.
    ///
    /// Fields extracted from Moment trait:
    /// - Vehicle state (gear, speed, RPM, shift point)
    /// - Flags (pit limiter, pit lane)
    ///
    /// Fields extracted from ACC physics:
    /// - Inputs (throttle, brake, clutch, steering angle)
    /// - Orientation (pitch, roll, yaw)
    /// - ABS status
    /// - Tire temperatures (core temperature and contact point temperatures)
    ///
    /// Fields extracted from ACC graphics:
    /// - Lap distance percentage
    /// - Lap number
    /// - Lap times
    ///
    /// Fields not available in ACC (set to None):
    /// - GPS coordinates (latitude_deg, longitude_deg)
    /// - Absolute lap distance (lap_distance_m)
    /// - Rate data (pitch_rate_rps, roll_rate_rps, yaw_rate_rps)
    /// - Acceleration data (lateral_accel_mps2, longitudinal_accel_mps2)
    #[cfg(windows)]
    pub fn from_acc_state(
        state: &simetry::assetto_corsa_competizione::SimState,
        point_no: usize,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        use uom::si::angular_velocity::revolution_per_minute;
        use uom::si::velocity::meter_per_second;

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // Extract base fields from Moment trait
        let gear = state.vehicle_gear();
        let speed_mps = state
            .vehicle_velocity()
            .map(|v| v.get::<meter_per_second>() as f32);
        let engine_rpm = state
            .vehicle_engine_rotation_speed()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32);
        let max_engine_rpm = state
            .vehicle_max_engine_rotation_speed()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32);
        let shift_point_rpm = state
            .shift_point()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32)
            .or_else(|| {
                // ACC doesn't provide shift_point through simetry API
                // Estimate optimal shift point as percentage of max RPM
                max_engine_rpm.map(|max_rpm| max_rpm * ACC_OPTIMAL_SHIFT_PCT)
            });

        // Extract inputs directly from ACC physics data
        // ACC provides these directly rather than through the Moment trait's pedals() method
        let throttle = Some(state.physics.gas);
        let brake = Some(state.physics.brake);
        let clutch = Some(state.physics.clutch);
        let steering_angle_rad = Some(state.physics.steer_angle);
        let steering_pct = Some(state.physics.steer_angle); // ACC uses normalized steering (-1.0 to 1.0)

        // Extract flags from Moment trait
        let is_pit_limiter_engaged = state.is_pit_limiter_engaged();
        let is_in_pit_lane = state.is_vehicle_in_pit_lane();

        // Extract position and lap data from ACC graphics
        let lap_distance_m = None; // ACC doesn't provide absolute lap distance
        let lap_distance_pct = Some(state.graphics.normalized_car_position);
        let lap_number = Some(state.graphics.completed_laps as u32);

        // Extract lap times from ACC graphics
        let last_lap_time_s = {
            let ms = state.graphics.lap_timing.last.millis;
            if ms > 0 {
                Some(ms as f32 / 1000.0)
            } else {
                None
            }
        };
        let best_lap_time_s = {
            let ms = state.graphics.lap_timing.best.millis;
            if ms > 0 {
                Some(ms as f32 / 1000.0)
            } else {
                None
            }
        };

        // Extract ABS status from ACC physics
        let is_abs_active = Some(state.physics.abs > 0.0);

        // GPS coordinates not available in ACC
        let latitude_deg = None;
        let longitude_deg = None;

        // Acceleration data not available in ACC
        let lateral_accel_mps2 = None;
        let longitudinal_accel_mps2 = None;

        // Extract orientation from ACC physics
        let pitch_rad = Some(state.physics.pitch);
        let roll_rad = Some(state.physics.roll);
        let yaw_rad = Some(state.physics.heading);

        // Rate data not available in ACC
        let pitch_rate_rps = None;
        let roll_rate_rps = None;
        let yaw_rate_rps = None;

        // Extract tire data from ACC physics WheelInfo
        // ACC provides tire temperatures through the wheels struct
        // According to simetry docs, WheelInfo has:
        // - tyre_core_temperature: single core temp value for carcass
        // - tyre_contact_point: Vector3<f32> with x, y, z representing contact point temps
        // We use tyre_contact_point.x/y/z for left/middle/right surface temps
        let lf_tire_info = Some(TireInfo {
            left_carcass_temp: state.physics.wheels.front_left.tyre_core_temperature,
            middle_carcass_temp: state.physics.wheels.front_left.tyre_core_temperature,
            right_carcass_temp: state.physics.wheels.front_left.tyre_core_temperature,
            left_surface_temp: state.physics.wheels.front_left.tyre_contact_point.x,
            middle_surface_temp: state.physics.wheels.front_left.tyre_contact_point.y,
            right_surface_temp: state.physics.wheels.front_left.tyre_contact_point.z,
        });

        let rf_tire_info = Some(TireInfo {
            left_carcass_temp: state.physics.wheels.front_right.tyre_core_temperature,
            middle_carcass_temp: state.physics.wheels.front_right.tyre_core_temperature,
            right_carcass_temp: state.physics.wheels.front_right.tyre_core_temperature,
            left_surface_temp: state.physics.wheels.front_right.tyre_contact_point.x,
            middle_surface_temp: state.physics.wheels.front_right.tyre_contact_point.y,
            right_surface_temp: state.physics.wheels.front_right.tyre_contact_point.z,
        });

        let lr_tire_info = Some(TireInfo {
            left_carcass_temp: state.physics.wheels.rear_left.tyre_core_temperature,
            middle_carcass_temp: state.physics.wheels.rear_left.tyre_core_temperature,
            right_carcass_temp: state.physics.wheels.rear_left.tyre_core_temperature,
            left_surface_temp: state.physics.wheels.rear_left.tyre_contact_point.x,
            middle_surface_temp: state.physics.wheels.rear_left.tyre_contact_point.y,
            right_surface_temp: state.physics.wheels.rear_left.tyre_contact_point.z,
        });

        let rr_tire_info = Some(TireInfo {
            left_carcass_temp: state.physics.wheels.rear_right.tyre_core_temperature,
            middle_carcass_temp: state.physics.wheels.rear_right.tyre_core_temperature,
            right_carcass_temp: state.physics.wheels.rear_right.tyre_core_temperature,
            left_surface_temp: state.physics.wheels.rear_right.tyre_contact_point.x,
            middle_surface_temp: state.physics.wheels.rear_right.tyre_contact_point.y,
            right_surface_temp: state.physics.wheels.rear_right.tyre_contact_point.z,
        });

        Self {
            point_no,
            timestamp_ms,
            game_source: GameSource::ACC,
            gear,
            speed_mps,
            engine_rpm,
            max_engine_rpm,
            shift_point_rpm,
            throttle,
            brake,
            clutch,
            steering_angle_rad,
            steering_pct,
            lap_distance_m,
            lap_distance_pct,
            lap_number,
            last_lap_time_s,
            best_lap_time_s,
            is_pit_limiter_engaged,
            is_in_pit_lane,
            is_abs_active,
            latitude_deg,
            longitude_deg,
            lateral_accel_mps2,
            longitudinal_accel_mps2,
            pitch_rad,
            pitch_rate_rps,
            roll_rad,
            roll_rate_rps,
            yaw_rad,
            yaw_rate_rps,
            lf_tire_info,
            rf_tire_info,
            lr_tire_info,
            rr_tire_info,
            annotations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TelemetryOutput {
    DataPoint(Box<TelemetryData>),
    SessionChange(SessionInfo),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub track_name: String,
    pub track_configuration: String,
    pub max_steering_angle: f32,
    pub track_length: String,
    pub game_source: GameSource,
    // Game-specific fields (may be None for some games)
    pub we_series_id: Option<i32>,
    pub we_session_id: Option<i32>,
    pub we_season_id: Option<i32>,
    pub we_sub_session_id: Option<i32>,
    pub we_league_id: Option<i32>,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            track_name: "Unknown".to_string(),
            track_configuration: "Unknown".to_string(),
            max_steering_angle: 0.,
            track_length: "".to_string(),
            game_source: GameSource::IRacing,
            we_series_id: None,
            we_session_id: None,
            we_season_id: None,
            we_sub_session_id: None,
            we_league_id: None,
        }
    }
}

/// Trait for analyzing telemetry data and detecting driving issues.
///
/// Analyzers process telemetry data to identify specific driving patterns or issues
/// such as slip, wheelspin, scrubbing, trail braking, and short shifting. Each analyzer
/// receives the unified `TelemetryData` representation, which provides access to all
/// available telemetry fields regardless of the source game.
///
/// # Requirements
///
/// This trait supports Requirements 1.2 and 5.1 by providing a game-agnostic interface
/// for telemetry analysis that works with the intermediate representation.
/// Trait for analyzing telemetry data and detecting driving issues.
///
/// Analyzers process telemetry data to identify specific driving patterns or issues
/// such as slip, wheelspin, scrubbing, trail braking, and short shifting. Each analyzer
/// receives the unified `TelemetryData` representation, which provides access to all
/// available telemetry fields regardless of the source game.
///
/// # Requirements
///
/// This trait supports Requirements 1.2 and 5.1 by providing a game-agnostic interface
/// for telemetry analysis that works with the intermediate representation.
pub trait TelemetryAnalyzer {
    /// Analyze telemetry data and return any detected annotations.
    ///
    /// # Arguments
    ///
    /// * `telemetry` - The telemetry data to analyze, containing all available fields
    ///   from the source game in a unified format
    /// * `session_info` - Information about the current racing session
    ///
    /// # Returns
    ///
    /// A vector of `TelemetryAnnotation` instances describing any detected issues or
    /// patterns in the telemetry data. Returns an empty vector if no issues are detected.
    fn analyze(
        &mut self,
        telemetry: &TelemetryData,
        session_info: &SessionInfo,
    ) -> Vec<TelemetryAnnotation>;
}

pub(crate) fn is_telemetry_point_analyzable(data: &TelemetryData) -> bool {
    !data.is_pit_limiter_engaged.unwrap_or(false) && data.speed_mps.unwrap_or(0.) > 0.
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for TelemetryData serialization and deserialization
    // Requirements: 7.1, 7.2, 7.3

    #[test]
    fn test_telemetry_data_serialization_with_all_fields() {
        // Create a TelemetryData instance with all fields populated
        let tire_info = TireInfo {
            left_carcass_temp: 80.0,
            middle_carcass_temp: 85.0,
            right_carcass_temp: 82.0,
            left_surface_temp: 90.0,
            middle_surface_temp: 95.0,
            right_surface_temp: 92.0,
        };

        let telemetry = TelemetryData {
            point_no: 42,
            timestamp_ms: 1234567890,
            game_source: GameSource::IRacing,
            gear: Some(3),
            speed_mps: Some(45.5),
            engine_rpm: Some(5500.0),
            max_engine_rpm: Some(7000.0),
            shift_point_rpm: Some(6500.0),
            throttle: Some(0.8),
            brake: Some(0.2),
            clutch: Some(0.0),
            steering_angle_rad: Some(0.5),
            steering_pct: Some(0.25),
            lap_distance_m: Some(1234.5),
            lap_distance_pct: Some(0.75),
            lap_number: Some(5),
            last_lap_time_s: Some(92.5),
            best_lap_time_s: Some(90.2),
            is_pit_limiter_engaged: Some(false),
            is_in_pit_lane: Some(false),
            is_abs_active: Some(true),
            latitude_deg: Some(37.7749),
            longitude_deg: Some(-122.4194),
            lateral_accel_mps2: Some(1.5),
            longitudinal_accel_mps2: Some(2.0),
            pitch_rad: Some(0.1),
            pitch_rate_rps: Some(0.05),
            roll_rad: Some(-0.2),
            roll_rate_rps: Some(-0.1),
            yaw_rad: Some(1.57),
            yaw_rate_rps: Some(0.3),
            lf_tire_info: Some(tire_info.clone()),
            rf_tire_info: Some(tire_info.clone()),
            lr_tire_info: Some(tire_info.clone()),
            rr_tire_info: Some(tire_info.clone()),
            annotations: Vec::new(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&telemetry).expect("Failed to serialize TelemetryData");

        // Verify JSON is not empty
        assert!(!json.is_empty());

        // Deserialize back
        let deserialized: TelemetryData =
            serde_json::from_str(&json).expect("Failed to deserialize TelemetryData");

        // Verify all fields match
        assert_eq!(deserialized.point_no, telemetry.point_no);
        assert_eq!(deserialized.timestamp_ms, telemetry.timestamp_ms);
        assert_eq!(deserialized.game_source, telemetry.game_source);
        assert_eq!(deserialized.gear, telemetry.gear);
        assert_eq!(deserialized.speed_mps, telemetry.speed_mps);
        assert_eq!(deserialized.engine_rpm, telemetry.engine_rpm);
        assert_eq!(deserialized.throttle, telemetry.throttle);
        assert_eq!(deserialized.brake, telemetry.brake);
        assert_eq!(
            deserialized.steering_angle_rad,
            telemetry.steering_angle_rad
        );
        assert_eq!(deserialized.lap_distance_m, telemetry.lap_distance_m);
        assert_eq!(deserialized.latitude_deg, telemetry.latitude_deg);
        assert_eq!(deserialized.longitude_deg, telemetry.longitude_deg);
        assert_eq!(deserialized.yaw_rate_rps, telemetry.yaw_rate_rps);
    }

    #[test]
    fn test_telemetry_data_serialization_with_none_fields() {
        // Create a TelemetryData instance with mostly None fields
        let telemetry = TelemetryData {
            point_no: 1,
            timestamp_ms: 1000,
            game_source: GameSource::ACC,
            gear: Some(2),
            speed_mps: Some(30.0),
            engine_rpm: None,
            max_engine_rpm: None,
            shift_point_rpm: None,
            throttle: None,
            brake: None,
            clutch: None,
            steering_angle_rad: None,
            steering_pct: None,
            lap_distance_m: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            is_abs_active: None,
            latitude_deg: None,
            longitude_deg: None,
            lateral_accel_mps2: None,
            longitudinal_accel_mps2: None,
            pitch_rad: None,
            pitch_rate_rps: None,
            roll_rad: None,
            roll_rate_rps: None,
            yaw_rad: None,
            yaw_rate_rps: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&telemetry).expect("Failed to serialize TelemetryData");

        // Verify JSON contains null for None fields
        assert!(json.contains("null"));

        // Deserialize back
        let deserialized: TelemetryData =
            serde_json::from_str(&json).expect("Failed to deserialize TelemetryData");

        // Verify None fields are preserved
        assert_eq!(deserialized.engine_rpm, None);
        assert_eq!(deserialized.throttle, None);
        assert_eq!(deserialized.steering_angle_rad, None);
        assert_eq!(deserialized.latitude_deg, None);
        assert_eq!(deserialized.yaw_rate_rps, None);
        assert_eq!(deserialized.lf_tire_info, None);

        // Verify Some fields are preserved
        assert_eq!(deserialized.gear, Some(2));
        assert_eq!(deserialized.speed_mps, Some(30.0));
    }

    #[test]
    fn test_telemetry_data_json_format() {
        // Create a simple TelemetryData instance
        let telemetry = TelemetryData {
            point_no: 10,
            timestamp_ms: 5000,
            game_source: GameSource::IRacing,
            gear: Some(4),
            speed_mps: Some(50.0),
            engine_rpm: Some(6000.0),
            ..Default::default()
        };

        // Serialize to pretty JSON for inspection
        let json =
            serde_json::to_string_pretty(&telemetry).expect("Failed to serialize TelemetryData");

        // Verify JSON contains expected fields
        assert!(json.contains("\"point_no\""));
        assert!(json.contains("\"timestamp_ms\""));
        assert!(json.contains("\"game_source\""));
        assert!(json.contains("\"gear\""));
        assert!(json.contains("\"speed_mps\""));
        assert!(json.contains("\"engine_rpm\""));

        // Verify JSON format is valid by deserializing
        let _: TelemetryData =
            serde_json::from_str(&json).expect("Failed to deserialize pretty JSON");
    }

    #[test]
    fn test_telemetry_data_deserialization_with_missing_optional_fields() {
        // Create JSON with only required fields and some optional fields
        let json = r#"{
            "point_no": 5,
            "timestamp_ms": 2000,
            "game_source": "ACC",
            "gear": 3,
            "speed_mps": 40.0,
            "engine_rpm": null,
            "max_engine_rpm": null,
            "shift_point_rpm": null,
            "throttle": null,
            "brake": null,
            "clutch": null,
            "steering_angle_rad": null,
            "steering_pct": null,
            "lap_distance_m": null,
            "lap_distance_pct": null,
            "lap_number": null,
            "last_lap_time_s": null,
            "best_lap_time_s": null,
            "is_pit_limiter_engaged": null,
            "is_in_pit_lane": null,
            "is_abs_active": null,
            "latitude_deg": null,
            "longitude_deg": null,
            "lateral_accel_mps2": null,
            "longitudinal_accel_mps2": null,
            "pitch_rad": null,
            "pitch_rate_rps": null,
            "roll_rad": null,
            "roll_rate_rps": null,
            "yaw_rad": null,
            "yaw_rate_rps": null,
            "lf_tire_info": null,
            "rf_tire_info": null,
            "lr_tire_info": null,
            "rr_tire_info": null,
            "annotations": []
        }"#;

        // Deserialize
        let telemetry: TelemetryData = serde_json::from_str(json)
            .expect("Failed to deserialize TelemetryData with missing fields");

        // Verify required fields
        assert_eq!(telemetry.point_no, 5);
        assert_eq!(telemetry.timestamp_ms, 2000);
        assert_eq!(telemetry.game_source, GameSource::ACC);

        // Verify optional fields are None
        assert_eq!(telemetry.engine_rpm, None);
        assert_eq!(telemetry.throttle, None);
        assert_eq!(telemetry.latitude_deg, None);

        // Verify Some fields
        assert_eq!(telemetry.gear, Some(3));
        assert_eq!(telemetry.speed_mps, Some(40.0));
    }

    #[test]
    fn test_tire_info_serialization() {
        let tire_info = TireInfo {
            left_carcass_temp: 75.0,
            middle_carcass_temp: 80.0,
            right_carcass_temp: 78.0,
            left_surface_temp: 85.0,
            middle_surface_temp: 90.0,
            right_surface_temp: 88.0,
        };

        // Serialize
        let json = serde_json::to_string(&tire_info).expect("Failed to serialize TireInfo");

        // Deserialize
        let deserialized: TireInfo =
            serde_json::from_str(&json).expect("Failed to deserialize TireInfo");

        // Verify all fields match
        assert_eq!(deserialized.left_carcass_temp, tire_info.left_carcass_temp);
        assert_eq!(
            deserialized.middle_carcass_temp,
            tire_info.middle_carcass_temp
        );
        assert_eq!(
            deserialized.right_carcass_temp,
            tire_info.right_carcass_temp
        );
        assert_eq!(deserialized.left_surface_temp, tire_info.left_surface_temp);
        assert_eq!(
            deserialized.middle_surface_temp,
            tire_info.middle_surface_temp
        );
        assert_eq!(
            deserialized.right_surface_temp,
            tire_info.right_surface_temp
        );
    }

    #[test]
    fn test_none_values_preserved_in_json() {
        // Create TelemetryData with mix of Some and None values
        let telemetry = TelemetryData {
            point_no: 100,
            timestamp_ms: 9999,
            game_source: GameSource::ACC,
            gear: Some(5),
            speed_mps: None, // Explicitly None
            engine_rpm: Some(7000.0),
            max_engine_rpm: None, // Explicitly None
            shift_point_rpm: None,
            throttle: Some(1.0),
            brake: None,
            clutch: None,
            steering_angle_rad: Some(0.3),
            steering_pct: None,
            lap_distance_m: None,
            lap_distance_pct: Some(0.5),
            lap_number: Some(10),
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: Some(false),
            is_in_pit_lane: None,
            is_abs_active: None,
            latitude_deg: None,
            longitude_deg: None,
            lateral_accel_mps2: None,
            longitudinal_accel_mps2: None,
            pitch_rad: None,
            pitch_rate_rps: None,
            roll_rad: None,
            roll_rate_rps: None,
            yaw_rad: None,
            yaw_rate_rps: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&telemetry).expect("Failed to serialize");

        // Verify that JSON contains null for None fields
        assert!(json.contains("\"speed_mps\":null"));
        assert!(json.contains("\"max_engine_rpm\":null"));
        assert!(json.contains("\"brake\":null"));
        assert!(json.contains("\"latitude_deg\":null"));

        // Verify that JSON contains values for Some fields
        assert!(json.contains("\"gear\":5"));
        assert!(json.contains("\"engine_rpm\":7000"));
        assert!(json.contains("\"throttle\":1"));

        // Deserialize and verify None values are preserved
        let deserialized: TelemetryData =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.speed_mps, None);
        assert_eq!(deserialized.max_engine_rpm, None);
        assert_eq!(deserialized.brake, None);
        assert_eq!(deserialized.latitude_deg, None);
        assert_eq!(deserialized.gear, Some(5));
        assert_eq!(deserialized.engine_rpm, Some(7000.0));
        assert_eq!(deserialized.throttle, Some(1.0));
    }

    #[test]
    fn test_new_annotation_types_serialization() {
        // Test EntryOversteer annotation
        let entry_oversteer = TelemetryAnnotation::EntryOversteer {
            expected_yaw_rate: 0.5,
            actual_yaw_rate: 0.8,
            is_oversteer: true,
        };
        let json = serde_json::to_string(&entry_oversteer).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, entry_oversteer);

        // Test MidCornerUndersteer annotation
        let mid_understeer = TelemetryAnnotation::MidCornerUndersteer {
            speed_loss: 2.5,
            is_understeer: true,
        };
        let json = serde_json::to_string(&mid_understeer).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, mid_understeer);

        // Test MidCornerOversteer annotation
        let mid_oversteer = TelemetryAnnotation::MidCornerOversteer {
            yaw_rate_excess: 0.3,
            is_oversteer: true,
        };
        let json = serde_json::to_string(&mid_oversteer).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, mid_oversteer);

        // Test FrontBrakeLock annotation
        let front_lock = TelemetryAnnotation::FrontBrakeLock {
            abs_activation_count: 3,
            is_front_lock: true,
        };
        let json = serde_json::to_string(&front_lock).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, front_lock);

        // Test RearBrakeLock annotation
        let rear_lock = TelemetryAnnotation::RearBrakeLock {
            abs_activation_count: 2,
            is_rear_lock: true,
        };
        let json = serde_json::to_string(&rear_lock).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, rear_lock);

        // Test TireOverheating annotation
        let tire_hot = TelemetryAnnotation::TireOverheating {
            avg_temp: 105.0,
            optimal_max: 95.0,
            is_overheating: true,
        };
        let json = serde_json::to_string(&tire_hot).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, tire_hot);

        // Test TireCold annotation
        let tire_cold = TelemetryAnnotation::TireCold {
            avg_temp: 70.0,
            optimal_min: 80.0,
            is_cold: true,
        };
        let json = serde_json::to_string(&tire_cold).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, tire_cold);

        // Test BottomingOut annotation
        let bottoming = TelemetryAnnotation::BottomingOut {
            pitch_change: 0.15,
            speed_loss: 3.0,
            is_bottoming: true,
        };
        let json = serde_json::to_string(&bottoming).expect("Failed to serialize");
        let deserialized: TelemetryAnnotation =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, bottoming);
    }

    #[test]
    fn test_new_annotation_types_display() {
        // Test Display implementation for new annotation types
        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::EntryOversteer {
                    expected_yaw_rate: 0.5,
                    actual_yaw_rate: 0.8,
                    is_oversteer: true,
                }
            ),
            "entry_oversteer"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::MidCornerUndersteer {
                    speed_loss: 2.5,
                    is_understeer: true,
                }
            ),
            "mid_corner_understeer"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::MidCornerOversteer {
                    yaw_rate_excess: 0.3,
                    is_oversteer: true,
                }
            ),
            "mid_corner_oversteer"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::FrontBrakeLock {
                    abs_activation_count: 3,
                    is_front_lock: true,
                }
            ),
            "front_brake_lock"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::RearBrakeLock {
                    abs_activation_count: 2,
                    is_rear_lock: true,
                }
            ),
            "rear_brake_lock"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::TireOverheating {
                    avg_temp: 105.0,
                    optimal_max: 95.0,
                    is_overheating: true,
                }
            ),
            "tire_overheating"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::TireCold {
                    avg_temp: 70.0,
                    optimal_min: 80.0,
                    is_cold: true,
                }
            ),
            "tire_cold"
        );

        assert_eq!(
            format!(
                "{}",
                TelemetryAnnotation::BottomingOut {
                    pitch_change: 0.15,
                    speed_loss: 3.0,
                    is_bottoming: true,
                }
            ),
            "bottoming_out"
        );
    }
}
