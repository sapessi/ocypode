use std::{
    collections::HashMap,
    sync::mpsc::Sender,
    thread,
    time::{Duration, SystemTime},
};

use crate::telemetry::TelemetryPoint;
use iracing::telemetry::{Sample, Value};
use iracing::Connection;

use crate::OcypodeError;

use super::{wheelspin_analyzer::WheelspinAnalyzer, TelemetryAnalyzer, TelemetryAnnotation};

const CONN_RETRY_WAIT_MS: u64 = 200;
const CONN_RETRY_MAX_WAIT_S: u64 = 600;
const REFRESH_RATE_MS: u64 = 100;
const MIN_WHEELSPIN_POINTS: usize = 500;

pub fn collect_telemetry(
    telemetry_sender: Sender<TelemetryPoint>,
    telemetry_writer_sender: Option<Sender<TelemetryPoint>>,
) -> Result<(), OcypodeError> {
    let start_time = SystemTime::now();
    let mut conn = Connection::new();
    while conn.is_err() {
        if SystemTime::now()
            .duration_since(start_time)
            .unwrap()
            .as_secs()
            >= CONN_RETRY_MAX_WAIT_S
        {
            println!(
                "Could not create iRacing connection after {} seconds",
                CONN_RETRY_MAX_WAIT_S
            );
            return Err(OcypodeError::IRacingConnectionTimeout);
        }
        thread::sleep(Duration::from_millis(CONN_RETRY_WAIT_MS));
        conn = Connection::new();
    }
    let client = conn.map_err(|e| OcypodeError::NoIRacingFile { source: e })?;
    let mut point_no: usize = 0;

    let mut analyzers = [WheelspinAnalyzer::<MIN_WHEELSPIN_POINTS>::new()];

    loop {
        thread::sleep(Duration::from_millis(REFRESH_RATE_MS));
        let telemetry_res = client.telemetry();
        if telemetry_res.is_err() {
            println!(
                "Could not retrieve telemetry: {}",
                telemetry_res.err().unwrap()
            );
            continue;
        }
        let telemetry = telemetry_res.unwrap();

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

        let mut measurement = TelemetryPoint {
            point_no,
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

        let mut annotations: HashMap<String, TelemetryAnnotation> = HashMap::new();
        for analyzer in analyzers.iter_mut() {
            annotations.extend(analyzer.analyze(&measurement));
        }
        if !annotations.is_empty() {
            measurement.annotations = annotations;
        }

        if point_no == usize::MAX {
            point_no = 0;
        }
        point_no += 1;

        telemetry_sender.send(measurement.clone()).map_err(|e| {
            println!(
                "Could not send telemetry point: {}\n\n {:#?}",
                e, measurement
            );
            OcypodeError::TelemetryBroadcastError { source: e }
        })?;
        if let Some(ref writer_sender) = telemetry_writer_sender {
            writer_sender.send(measurement.clone()).map_err(|e| {
                println!(
                    "Could not send telemetry point: {}\n\n {:#?}",
                    e, measurement
                );
                OcypodeError::TelemetryBroadcastError { source: e }
            })?;
        }
    }
}

fn get_float(telemetry_sample: &Sample, name: &'static str) -> f32 {
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

fn get_int(telemetry_sample: &Sample, name: &'static str) -> u32 {
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

fn get_bool(telemetry_sample: &Sample, name: &'static str) -> bool {
    telemetry_sample
        .get(name)
        .unwrap_or_else(|e| {
            println!("Error retrieve telemetry sample for {}: {}", name, e);
            Value::BOOL(false)
        })
        .into()
}
