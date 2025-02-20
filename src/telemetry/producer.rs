use std::{
    collections::HashMap,
    thread,
    time::{Duration, SystemTime},
};

use iracing::{
    telemetry::{Sample, Value},
    Connection,
};

use crate::OcypodeError;

use super::{SessionInfo, TelemetryPoint};

const CONN_RETRY_WAIT_MS: u64 = 200;
const CONN_RETRY_MAX_WAIT_S: u64 = 600;

/// A trait for producing telemetry data that abstracts away
/// the iRacing client so that we can test collection and analyzers
/// offline
pub trait TelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError>;
    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError>;
    fn telemetry(&mut self) -> Result<TelemetryPoint, OcypodeError>;
}

pub(crate) struct IRacingTelemetryProducer {
    client: Option<iracing::Connection>,
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
            max_steering_angle: get_float(&telemetry, "SteeringWheelAngleMax"),
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

        let lap_dist_pct: f32 = get_float(&telemetry, "LapDistPct");
        let lap_no: u32 = get_int(&telemetry, "Lap");
        let last_lap_time_s: f32 = get_float(&telemetry, "LapLastLapTime");
        let car_shift_ideal_rpm: f32 = get_float(&telemetry, "PlayerCarSLShiftRPM");
        let cur_gear: u32 = get_int(&telemetry, "Gear");
        let cur_rpm: f32 = get_float(&telemetry, "RPM");
        let cur_speed: f32 = get_float(&telemetry, "Speed");
        let best_lap_time_s: f32 = get_float(&telemetry, "LapBestLapTime");
        let throttle: f32 = get_float(&telemetry, "Throttle");
        let brake: f32 = get_float(&telemetry, "BrakeRaw");
        let steering: f32 = get_float(&telemetry, "SteeringWheelAngle");
        let abs_active: bool = get_bool(&telemetry, "BrakeABSactive");

        let measurement = TelemetryPoint {
            point_no: self.point_no,
            lap_dist_pct,
            lap_no,
            last_lap_time_s,
            best_lap_time_s,
            car_shift_ideal_rpm,
            cur_gear,
            cur_rpm,
            cur_speed,
            throttle,
            brake,
            steering,
            abs_active,
            annotations: HashMap::new(),
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
            max_steering_angle: self.max_steering_angle,
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

pub(crate) fn get_float(telemetry_sample: &Sample, name: &'static str) -> f32 {
    telemetry_sample
        .get(name)
        .unwrap_or_else(|e| {
            println!("Error retrieve telemetry sample for {}: {}", name, e);
            Value::FLOAT(0.)
        })
        .try_into()
        .unwrap_or_else(|e| {
            println!("Error while parsing float for {}: {}", name, e);
            0.
        })
}

pub(crate) fn get_int(telemetry_sample: &Sample, name: &'static str) -> u32 {
    telemetry_sample
        .get(name)
        .unwrap_or_else(|e| {
            println!("Error retrieve telemetry sample for {}: {}", name, e);
            Value::INT(0)
        })
        .try_into()
        .unwrap_or_else(|e| {
            println!("Error while parsing float for {}: {}", name, e);
            0
        })
}

pub(crate) fn get_bool(telemetry_sample: &Sample, name: &'static str) -> bool {
    telemetry_sample
        .get(name)
        .unwrap_or_else(|e| {
            println!("Error retrieve telemetry sample for {}: {}", name, e);
            Value::BOOL(false)
        })
        .into()
}
