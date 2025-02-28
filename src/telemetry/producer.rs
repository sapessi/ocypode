use std::{
    thread,
    time::{Duration, SystemTime},
};

use iracing::telemetry::{Connection, Sample};

use crate::OcypodeError;

use super::{SessionInfo, TelemetryPoint, TireInfo};

const CONN_RETRY_WAIT_MS: u64 = 200;
const MAX_STEERING_ANGLE_DEFAULT: f32 = std::f32::consts::PI;
pub(crate) const CONN_RETRY_MAX_WAIT_S: u64 = 600;

pub trait SimplifiedTelemetryAccess {
    fn get_float(&self, name: &'static str) -> Option<f32>;
    fn get_int(&self, name: &'static str) -> Option<u32>;
    fn get_bool(&self, name: &'static str) -> Option<bool>;
    fn get_lf_tire_info(&self) -> Option<TireInfo>;
    fn get_rf_tire_info(&self) -> Option<TireInfo>;
    fn get_lr_tire_info(&self) -> Option<TireInfo>;
    fn get_rr_tire_info(&self) -> Option<TireInfo>;
}

impl SimplifiedTelemetryAccess for Sample {
    fn get_float(&self, name: &'static str) -> Option<f32> {
        if let Ok(value) = self.get(name) {
            return value.try_into().ok();
        }
        None
    }

    fn get_int(&self, name: &'static str) -> Option<u32> {
        if let Ok(value) = self.get(name) {
            return value.try_into().ok();
        }
        None
    }

    fn get_bool(&self, name: &'static str) -> Option<bool> {
        if let Ok(value) = self.get(name) {
            return Some(value.into());
        }
        None
    }

    fn get_lf_tire_info(&self) -> Option<TireInfo> {
        if self.has("LFtempCL") {
            return Some(TireInfo {
                left_carcass_temp: self.get_float("LFtempCL").unwrap_or(0.),
                middle_carcass_temp: self.get_float("LFtempCM").unwrap_or(0.),
                right_carcass_temp: self.get_float("LFtempCR").unwrap_or(0.),
                left_surface_temp: self.get_float("LFtempL").unwrap_or(0.),
                middle_surface_temp: self.get_float("LFtempM").unwrap_or(0.),
                right_surface_temp: self.get_float("LFtempR").unwrap_or(0.),
            });
        }
        None
    }

    fn get_rf_tire_info(&self) -> Option<TireInfo> {
        if self.has("LFtempCL") {
            return Some(TireInfo {
                left_carcass_temp: self.get_float("RFtempCL").unwrap_or(0.),
                middle_carcass_temp: self.get_float("RFtempCM").unwrap_or(0.),
                right_carcass_temp: self.get_float("RFtempCR").unwrap_or(0.),
                left_surface_temp: self.get_float("RFtempL").unwrap_or(0.),
                middle_surface_temp: self.get_float("RFtempM").unwrap_or(0.),
                right_surface_temp: self.get_float("RFtempR").unwrap_or(0.),
            });
        }
        None
    }

    fn get_lr_tire_info(&self) -> Option<TireInfo> {
        if self.has("LFtempCL") {
            return Some(TireInfo {
                left_carcass_temp: self.get_float("LRtempCL").unwrap_or(0.),
                middle_carcass_temp: self.get_float("LRtempCM").unwrap_or(0.),
                right_carcass_temp: self.get_float("LRtempCR").unwrap_or(0.),
                left_surface_temp: self.get_float("LRtempL").unwrap_or(0.),
                middle_surface_temp: self.get_float("LRtempM").unwrap_or(0.),
                right_surface_temp: self.get_float("LRtempR").unwrap_or(0.),
            });
        }
        None
    }

    fn get_rr_tire_info(&self) -> Option<TireInfo> {
        if self.has("LFtempCL") {
            return Some(TireInfo {
                left_carcass_temp: self.get_float("RRtempCL").unwrap_or(0.),
                middle_carcass_temp: self.get_float("RRtempCM").unwrap_or(0.),
                right_carcass_temp: self.get_float("RRtempCR").unwrap_or(0.),
                left_surface_temp: self.get_float("RRtempL").unwrap_or(0.),
                middle_surface_temp: self.get_float("RRtempM").unwrap_or(0.),
                right_surface_temp: self.get_float("RRtempR").unwrap_or(0.),
            });
        }
        None
    }
}

/// A trait for producing telemetry data that abstracts away
/// the iRacing client so that we can test collection and analyzers
/// offline
pub trait TelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError>;
    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError>;
    fn telemetry(&mut self) -> Result<TelemetryPoint, OcypodeError>;
}

pub(crate) struct IRacingTelemetryProducer {
    client: Option<Connection>,
    retry_wait_ms: u64,
    retry_timeout_s: u64,
    point_no: usize,
}

impl Default for IRacingTelemetryProducer {
    fn default() -> Self {
        IRacingTelemetryProducer::new(CONN_RETRY_WAIT_MS, CONN_RETRY_MAX_WAIT_S)
    }
}

impl IRacingTelemetryProducer {
    pub fn new(retry_wait_ms: u64, retry_timeout_s: u64) -> Self {
        Self {
            client: None,
            retry_wait_ms,
            retry_timeout_s,
            point_no: 0,
        }
    }
}

impl TelemetryProducer for IRacingTelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError> {
        let start_time = SystemTime::now();
        let mut conn = Connection::new();
        while conn.is_err() {
            if SystemTime::now()
                .duration_since(start_time)
                .unwrap()
                .as_secs()
                >= self.retry_timeout_s
            {
                println!(
                    "Could not create iRacing connection after {} seconds",
                    self.retry_timeout_s
                );
                return Err(OcypodeError::IRacingConnectionTimeout);
            }
            thread::sleep(Duration::from_millis(self.retry_wait_ms));
            conn = Connection::new();
        }
        self.client = Some(conn.map_err(|e| OcypodeError::NoIRacingFile { source: e })?);
        Ok(())
    }

    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The iRacing connection is not initialized, call start() first.",
            });
        }
        let ir_session_info = self
            .client
            .as_mut()
            .expect("Missing iRacing connection")
            .session_info()
            .map_err(|e| {
                println!("Could not retrieve session info: {}", e);
                OcypodeError::TelemetryProducerError {
                    description: "Could not retrieve session info",
                }
            })?;
        let telemetry = self
            .client
            .as_ref()
            .expect("Missing iRacing connection")
            .telemetry()
            .map_err(|e| {
                println!("Could not retrieve telemetry: {}", e);
                OcypodeError::TelemetryProducerError {
                    description: "Could not retrieve telemetry",
                }
            })?;

        Ok(SessionInfo {
            track_name: ir_session_info.weekend.track_name,
            track_configuration: ir_session_info.weekend.track_config_name,
            max_steering_angle: telemetry.get_float("SteeringWheelAngleMax").unwrap_or(0.),
            track_length: ir_session_info.weekend.track_length,
            we_series_id: ir_session_info.weekend.series_id,
            we_session_id: ir_session_info.weekend.session_id,
            we_season_id: ir_session_info.weekend.season_id,
            we_sub_session_id: ir_session_info.weekend.sub_session_id,
            we_league_id: ir_session_info.weekend.league_id,
        })
    }

    fn telemetry(&mut self) -> Result<TelemetryPoint, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The iRacing connection is not initialized, call start() first.",
            });
        }
        let telemetry = self
            .client
            .as_ref()
            .expect("Missing iRacing connection")
            .telemetry()
            .map_err(|e| {
                println!("Could not retrieve telemetry: {}", e);
                OcypodeError::TelemetryProducerError {
                    description: "Could not retrieve telemetry",
                }
            })?;

        let steering_pct = if telemetry.get_float("SteeringWheelAngle").unwrap_or(0.)
            > MAX_STEERING_ANGLE_DEFAULT
        {
            telemetry.get_float("SteeringWheelAngle").unwrap_or(0.)
                / telemetry.get_float("SteeringWheelAngleMax").unwrap_or(0.)
        } else {
            telemetry.get_float("SteeringWheelAngle").unwrap_or(0.) / MAX_STEERING_ANGLE_DEFAULT
        };

        let measurement = TelemetryPoint {
            point_no: self.point_no,
            lap_dist: telemetry.get_float("LapDist").unwrap_or(0.),
            lap_dist_pct: telemetry.get_float("LapDistPct").unwrap_or(0.),
            lap_no: telemetry.get_int("Lap").unwrap_or(0),
            last_lap_time_s: telemetry.get_float("LapLastLapTime").unwrap_or(0.),
            best_lap_time_s: telemetry.get_float("LapBestLapTime").unwrap_or(0.),
            car_shift_ideal_rpm: telemetry.get_float("PlayerCarSLShiftRPM").unwrap_or(0.),
            cur_gear: telemetry.get_int("Gear").unwrap_or(0),
            cur_rpm: telemetry.get_float("RPM").unwrap_or(0.),
            cur_speed: telemetry.get_float("Speed").unwrap_or(0.),
            throttle: telemetry.get_float("Throttle").unwrap_or(0.),
            brake: telemetry.get_float("BrakeRaw").unwrap_or(0.),
            steering: telemetry.get_float("SteeringWheelAngle").unwrap_or(0.),
            // this might result in an unhappy division by 0. Do we want to panic in this case because it's unexpected?
            steering_pct,
            abs_active: telemetry.get_bool("BrakeABSactive").unwrap_or(false),
            lat: telemetry.get_float("Lat").unwrap_or(0.),
            lon: telemetry.get_float("Lon").unwrap_or(0.),
            lat_accel: telemetry.get_float("LatAccel").unwrap_or(0.),
            lon_accel: telemetry.get_float("LonAccel").unwrap_or(0.),
            pitch: telemetry.get_float("Pitch").unwrap_or(0.),
            pitch_rate: telemetry.get_float("PitchRate").unwrap_or(0.),
            roll: telemetry.get_float("Roll").unwrap_or(0.),
            roll_rate: telemetry.get_float("RollRate").unwrap_or(0.),
            yaw: telemetry.get_float("Yaw").unwrap_or(0.),
            yaw_rate: telemetry.get_float("YawRate").unwrap_or(0.),
            lf_tire_info: telemetry.get_lf_tire_info(),
            rf_tire_info: telemetry.get_rf_tire_info(),
            lr_tire_info: telemetry.get_lr_tire_info(),
            rr_tire_info: telemetry.get_rr_tire_info(),
            ..Default::default()
        };
        if self.point_no == usize::MAX {
            self.point_no = 0;
        }
        self.point_no += 1;
        Ok(measurement)
    }
}

pub(crate) struct MockTelemetryProducer {
    cur_tick: usize,
    points: Vec<TelemetryPoint>,
    track_name: String,
    max_steering_angle: f32,
}

impl Default for MockTelemetryProducer {
    fn default() -> Self {
        Self {
            cur_tick: 0,
            points: Vec::new(),
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
        }
    }
}

#[allow(dead_code)]
impl MockTelemetryProducer {
    pub fn from_points(points: Vec<TelemetryPoint>) -> Self {
        Self {
            cur_tick: 0,
            points,
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
        }
    }

    pub fn from_file(file: &str) -> Result<Self, OcypodeError> {
        let file =
            std::fs::File::open(file).map_err(|e| OcypodeError::NoIRacingFile { source: e })?;
        let reader = std::io::BufReader::new(file);
        let points: Vec<TelemetryPoint> = serde_json::from_reader(reader).map_err(|e| {
            println!("Could not load JSON file: {}", e);
            OcypodeError::TelemetryProducerError {
                description: "Could not load JSON file",
            }
        })?;

        Ok(Self {
            cur_tick: 0,
            points,
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
        })
    }
}

impl TelemetryProducer for MockTelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError> {
        Ok(())
    }

    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError> {
        Ok(SessionInfo {
            track_name: self.track_name.clone(),
            track_configuration: String::new(),
            max_steering_angle: self.max_steering_angle,
            track_length: "1.5".to_string(),
            we_series_id: 0,
            we_session_id: 0,
            we_season_id: 0,
            we_sub_session_id: 0,
            we_league_id: 0,
        })
    }

    fn telemetry(&mut self) -> Result<TelemetryPoint, OcypodeError> {
        self.cur_tick += 1;
        if self.points.len() < self.cur_tick {
            return Err(OcypodeError::TelemetryProducerError {
                description: "End of points vec",
            });
        }
        Ok(self.points[self.cur_tick - 1].clone())
    }
}
