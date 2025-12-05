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
use simetry::Moment;

/// A mock implementation of the Moment trait for testing purposes.
/// 
/// This struct wraps SerializableTelemetry data and implements the simetry Moment trait,
/// allowing MockTelemetryProducer to return telemetry data in the same format as live
/// game producers while using pre-recorded or generated test data.
#[derive(Clone, Debug)]
pub(crate) struct MockMoment {
    gear: Option<i8>,
    velocity: Option<uom::si::f64::Velocity>,
    engine_rpm: Option<uom::si::f64::AngularVelocity>,
    max_engine_rpm: Option<uom::si::f64::AngularVelocity>,
    shift_point: Option<uom::si::f64::AngularVelocity>,
    pedals: Option<simetry::Pedals>,
    pit_limiter: Option<bool>,
    in_pit_lane: Option<bool>,
}

impl MockMoment {
    pub(crate) fn new(data: SerializableTelemetry) -> Self {
        use uom::si::angular_velocity::revolution_per_minute;
        use uom::si::velocity::meter_per_second;
        
        let velocity = data.speed_mps.map(|speed| {
            uom::si::f64::Velocity::new::<meter_per_second>(speed as f64)
        });
        
        let engine_rpm = data.engine_rpm.map(|rpm| {
            uom::si::f64::AngularVelocity::new::<revolution_per_minute>(rpm as f64)
        });
        
        let max_engine_rpm = data.max_engine_rpm.map(|rpm| {
            uom::si::f64::AngularVelocity::new::<revolution_per_minute>(rpm as f64)
        });
        
        let shift_point = data.shift_point_rpm.map(|rpm| {
            uom::si::f64::AngularVelocity::new::<revolution_per_minute>(rpm as f64)
        });
        
        let pedals = match (data.throttle, data.brake, data.clutch) {
            (Some(throttle), Some(brake), Some(clutch)) => Some(simetry::Pedals {
                throttle: throttle as f64,
                brake: brake as f64,
                clutch: clutch as f64,
            }),
            _ => None,
        };
        
        Self {
            gear: data.gear,
            velocity,
            engine_rpm,
            max_engine_rpm,
            shift_point,
            pedals,
            pit_limiter: data.is_pit_limiter_engaged,
            in_pit_lane: data.is_in_pit_lane,
        }
    }
}

impl Moment for MockMoment {
    fn vehicle_gear(&self) -> Option<i8> {
        self.gear
    }

    fn vehicle_velocity(&self) -> Option<uom::si::f64::Velocity> {
        self.velocity
    }

    fn vehicle_engine_rotation_speed(&self) -> Option<uom::si::f64::AngularVelocity> {
        self.engine_rpm
    }

    fn vehicle_max_engine_rotation_speed(&self) -> Option<uom::si::f64::AngularVelocity> {
        self.max_engine_rpm
    }

    fn shift_point(&self) -> Option<uom::si::f64::AngularVelocity> {
        self.shift_point
    }

    fn pedals(&self) -> Option<simetry::Pedals> {
        self.pedals.clone()
    }

    fn is_pit_limiter_engaged(&self) -> Option<bool> {
        self.pit_limiter
    }

    fn is_vehicle_in_pit_lane(&self) -> Option<bool> {
        self.in_pit_lane
    }
}

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameSource {
    IRacing,
    ACC,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableTelemetry {
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
    pub steering: Option<f32>,
    pub steering_pct: Option<f32>,
    
    // Position and orientation
    pub lap_distance: Option<f32>,
    pub lap_distance_pct: Option<f32>,
    pub lap_number: Option<u32>,
    
    // Lap times
    pub last_lap_time_s: Option<f32>,
    pub best_lap_time_s: Option<f32>,
    
    // Flags and states
    pub is_pit_limiter_engaged: Option<bool>,
    pub is_in_pit_lane: Option<bool>,
    pub abs_active: Option<bool>,
    
    // Position data
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    
    // Acceleration
    pub lat_accel: Option<f32>,
    pub lon_accel: Option<f32>,
    
    // Orientation
    pub pitch: Option<f32>,
    pub pitch_rate: Option<f32>,
    pub roll: Option<f32>,
    pub roll_rate: Option<f32>,
    pub yaw: Option<f32>,
    pub yaw_rate: Option<f32>,
    
    // Tire data
    pub lf_tire_info: Option<TireInfo>,
    pub rf_tire_info: Option<TireInfo>,
    pub lr_tire_info: Option<TireInfo>,
    pub rr_tire_info: Option<TireInfo>,
    
    // Annotations from analyzers
    pub annotations: Vec<TelemetryAnnotation>,
}

impl Default for SerializableTelemetry {
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
            steering: None,
            steering_pct: None,
            lap_distance: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            abs_active: None,
            lat: None,
            lon: None,
            lat_accel: None,
            lon_accel: None,
            pitch: None,
            pitch_rate: None,
            roll: None,
            roll_rate: None,
            yaw: None,
            yaw_rate: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        }
    }
}

impl SerializableTelemetry {
    pub fn from_moment(
        moment: &dyn Moment,
        point_no: usize,
        game_source: GameSource,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        // Extract vehicle state
        let gear = moment.vehicle_gear();
        // Convert uom Velocity to f32 m/s
        let speed_mps = moment.vehicle_velocity().map(|v| {
            use uom::si::velocity::meter_per_second;
            v.get::<meter_per_second>() as f32
        });
        // Convert uom AngularVelocity to f32 RPM
        let engine_rpm = moment.vehicle_engine_rotation_speed().map(|rpm| {
            use uom::si::angular_velocity::revolution_per_minute;
            rpm.get::<revolution_per_minute>() as f32
        });
        let max_engine_rpm = moment.vehicle_max_engine_rotation_speed().map(|rpm| {
            use uom::si::angular_velocity::revolution_per_minute;
            rpm.get::<revolution_per_minute>() as f32
        });
        let shift_point_rpm = moment.shift_point().map(|rpm| {
            use uom::si::angular_velocity::revolution_per_minute;
            rpm.get::<revolution_per_minute>() as f32
        });
        
        // Extract inputs
        let pedals = moment.pedals();
        let throttle = pedals.as_ref().map(|p| p.throttle as f32);
        let brake = pedals.as_ref().map(|p| p.brake as f32);
        let clutch = pedals.as_ref().map(|p| p.clutch as f32);
        
        // Note: simetry's Moment trait doesn't have steering, lap distance, lap times, etc.
        // These will need to be extracted from game-specific implementations
        // For now, we set them to None as the base trait doesn't provide them
        let steering = None;
        let steering_pct = None;
        let lap_distance = None;
        let lap_distance_pct = None;
        let lap_number = None;
        let last_lap_time_s = None;
        let best_lap_time_s = None;
        
        // Extract flags and states
        let is_pit_limiter_engaged = moment.is_pit_limiter_engaged();
        let is_in_pit_lane = moment.is_vehicle_in_pit_lane();
        let abs_active = None; // Not in base Moment trait
        
        // Position, acceleration, orientation not in base Moment trait
        let lat = None;
        let lon = None;
        let lat_accel = None;
        let lon_accel = None;
        let pitch = None;
        let pitch_rate = None;
        let roll = None;
        let roll_rate = None;
        let yaw = None;
        let yaw_rate = None;
        
        // Tire data not in base Moment trait
        let lf_tire_info = None;
        let rf_tire_info = None;
        let lr_tire_info = None;
        let rr_tire_info = None;
        
        Self {
            point_no,
            timestamp_ms,
            game_source,
            gear,
            speed_mps,
            engine_rpm,
            max_engine_rpm,
            shift_point_rpm,
            throttle,
            brake,
            clutch,
            steering,
            steering_pct,
            lap_distance,
            lap_distance_pct,
            lap_number,
            last_lap_time_s,
            best_lap_time_s,
            is_pit_limiter_engaged,
            is_in_pit_lane,
            abs_active,
            lat,
            lon,
            lat_accel,
            lon_accel,
            pitch,
            pitch_rate,
            roll,
            roll_rate,
            yaw,
            yaw_rate,
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
    DataPoint(SerializableTelemetry),
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

pub trait TelemetryAnalyzer {
    fn analyze(
        &mut self,
        telemetry: &dyn Moment,
        session_info: &SessionInfo,
    ) -> Vec<TelemetryAnnotation>;
}

#[cfg(test)]

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use simetry::Moment;

    // Mock implementation of Moment trait for testing
    #[derive(Clone, Debug)]
    struct MockMoment {
        gear: Option<i8>,
        velocity: Option<uom::si::f64::Velocity>,
        engine_rpm: Option<uom::si::f64::AngularVelocity>,
        max_engine_rpm: Option<uom::si::f64::AngularVelocity>,
        shift_point: Option<uom::si::f64::AngularVelocity>,
        pedals: Option<simetry::Pedals>,
        pit_limiter: Option<bool>,
        in_pit_lane: Option<bool>,
    }

    impl Moment for MockMoment {
        fn vehicle_gear(&self) -> Option<i8> {
            self.gear
        }

        fn vehicle_velocity(&self) -> Option<uom::si::f64::Velocity> {
            self.velocity
        }

        fn vehicle_engine_rotation_speed(&self) -> Option<uom::si::f64::AngularVelocity> {
            self.engine_rpm
        }

        fn vehicle_max_engine_rotation_speed(&self) -> Option<uom::si::f64::AngularVelocity> {
            self.max_engine_rpm
        }

        fn shift_point(&self) -> Option<uom::si::f64::AngularVelocity> {
            self.shift_point
        }

        fn pedals(&self) -> Option<simetry::Pedals> {
            self.pedals.clone()
        }

        fn is_pit_limiter_engaged(&self) -> Option<bool> {
            self.pit_limiter
        }

        fn is_vehicle_in_pit_lane(&self) -> Option<bool> {
            self.in_pit_lane
        }
    }

    // Property test generators
    fn arb_velocity() -> impl Strategy<Value = uom::si::f64::Velocity> {
        any::<f64>().prop_map(|v| uom::si::f64::Velocity::new::<uom::si::velocity::meter_per_second>(v))
    }

    fn arb_angular_velocity() -> impl Strategy<Value = uom::si::f64::AngularVelocity> {
        any::<f64>().prop_map(|rpm| uom::si::f64::AngularVelocity::new::<uom::si::angular_velocity::revolution_per_minute>(rpm))
    }

    fn arb_pedals() -> impl Strategy<Value = simetry::Pedals> {
        (0.0f64..=1.0, 0.0f64..=1.0, 0.0f64..=1.0)
            .prop_map(|(throttle, brake, clutch)| simetry::Pedals { throttle, brake, clutch })
    }

    fn arb_mock_moment() -> impl Strategy<Value = MockMoment> {
        (
            prop::option::of(any::<i8>()),
            prop::option::of(arb_velocity()),
            prop::option::of(arb_angular_velocity()),
            prop::option::of(arb_angular_velocity()),
            prop::option::of(arb_angular_velocity()),
            prop::option::of(arb_pedals()),
            prop::option::of(any::<bool>()),
            prop::option::of(any::<bool>()),
        )
            .prop_map(|(
                gear,
                velocity,
                engine_rpm,
                max_engine_rpm,
                shift_point,
                pedals,
                pit_limiter,
                in_pit_lane,
            )| MockMoment {
                gear,
                velocity,
                engine_rpm,
                max_engine_rpm,
                shift_point,
                pedals,
                pit_limiter,
                in_pit_lane,
            })
    }

    // **Feature: multi-game-telemetry-support, Property 1: iRacing data field extraction completeness**
    // **Validates: Requirements 2.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn test_iracing_data_field_extraction_completeness(
            moment in arb_mock_moment(),
            point_no in any::<usize>(),
        ) {
            // For any iRacing telemetry data received from simetry, all previously supported 
            // data fields should be successfully extractable from simetry's data structures
            
            let serializable = SerializableTelemetry::from_moment(
                &moment,
                point_no,
                GameSource::IRacing,
            );
            
            // Verify that the conversion doesn't panic and produces a valid result
            assert_eq!(serializable.point_no, point_no);
            assert_eq!(serializable.game_source, GameSource::IRacing);
            
            // Verify that all fields that were present in the moment are extracted
            // (or None if not present)
            
            // Vehicle state fields
            assert_eq!(serializable.gear, moment.vehicle_gear());
            
            if let Some(velocity) = moment.vehicle_velocity() {
                let expected_speed = velocity.get::<uom::si::velocity::meter_per_second>() as f32;
                assert_eq!(serializable.speed_mps, Some(expected_speed));
            } else {
                assert_eq!(serializable.speed_mps, None);
            }
            
            if let Some(rpm) = moment.vehicle_engine_rotation_speed() {
                let expected_rpm = rpm.get::<uom::si::angular_velocity::revolution_per_minute>() as f32;
                assert_eq!(serializable.engine_rpm, Some(expected_rpm));
            } else {
                assert_eq!(serializable.engine_rpm, None);
            }
            
            if let Some(rpm) = moment.vehicle_max_engine_rotation_speed() {
                let expected_rpm = rpm.get::<uom::si::angular_velocity::revolution_per_minute>() as f32;
                assert_eq!(serializable.max_engine_rpm, Some(expected_rpm));
            } else {
                assert_eq!(serializable.max_engine_rpm, None);
            }
            
            if let Some(rpm) = moment.shift_point() {
                let expected_rpm = rpm.get::<uom::si::angular_velocity::revolution_per_minute>() as f32;
                assert_eq!(serializable.shift_point_rpm, Some(expected_rpm));
            } else {
                assert_eq!(serializable.shift_point_rpm, None);
            }
            
            // Input fields
            if let Some(pedals) = moment.pedals() {
                assert_eq!(serializable.throttle, Some(pedals.throttle as f32));
                assert_eq!(serializable.brake, Some(pedals.brake as f32));
                assert_eq!(serializable.clutch, Some(pedals.clutch as f32));
            } else {
                assert_eq!(serializable.throttle, None);
                assert_eq!(serializable.brake, None);
                assert_eq!(serializable.clutch, None);
            }
            
            // Flags
            assert_eq!(serializable.is_pit_limiter_engaged, moment.is_pit_limiter_engaged());
            assert_eq!(serializable.is_in_pit_lane, moment.is_vehicle_in_pit_lane());
            
            // Verify annotations are initialized as empty
            assert!(serializable.annotations.is_empty());
        }
    }
}
