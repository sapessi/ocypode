use std::{
    collections::HashMap,
    sync::mpsc::Sender,
    thread,
    time::{Duration, SystemTime},
};

use crate::telemetry::TelemetryPoint;

use crate::OcypodeError;

use super::{
    producer::{TelemetryProducer, CONN_RETRY_MAX_WAIT_S}, short_shifting_analyzer::ShortShiftingAnalyzer, slip_analyzer::SlipAnalyzer, trailbrake_steering_analyzer::{
        TrailbrakeSteeringAnalyzer, MAX_TRAILBRAKING_STEERING_ANGLE, MIN_TRAILBRAKING_PCT,
    }, wheelspin_analyzer::WheelspinAnalyzer, TelemetryAnalyzer, TelemetryAnnotation
};

const REFRESH_RATE_MS: u64 = 100;
const MIN_WHEELSPIN_POINTS: usize = 100;

pub fn collect_telemetry(
    mut producer: impl TelemetryProducer,
    telemetry_sender: Sender<TelemetryPoint>,
    telemetry_writer_sender: Option<Sender<TelemetryPoint>>,
) -> Result<(), OcypodeError> {
    producer.start()?;

    // wait for a session to start
    let session_wait_start = SystemTime::now();
    loop {
        if producer.session_info().is_err() {
            thread::sleep(Duration::from_millis(REFRESH_RATE_MS));
        } else {
            break;
        }
        if SystemTime::now()
            .duration_since(session_wait_start)
            .unwrap()
            .as_secs()
            > CONN_RETRY_MAX_WAIT_S
        {
            return Err(OcypodeError::IRacingConnectionTimeout);
        }
    }

    let mut analyzers: Vec<Box<dyn TelemetryAnalyzer>> = vec![
        Box::new(WheelspinAnalyzer::<MIN_WHEELSPIN_POINTS>::new()),
        Box::new(TrailbrakeSteeringAnalyzer::new(
            MAX_TRAILBRAKING_STEERING_ANGLE,
            MIN_TRAILBRAKING_PCT,
        )),
        Box::new(ShortShiftingAnalyzer::default()),
        Box::new(SlipAnalyzer::default()),
    ];

    loop {
        thread::sleep(Duration::from_millis(REFRESH_RATE_MS));
        let mut measurement = producer.telemetry()?;

        let mut annotations: HashMap<String, TelemetryAnnotation> = HashMap::new();
        for analyzer in analyzers.iter_mut() {
            annotations.extend(analyzer.analyze(&measurement, &producer.session_info()?));
        }
        if !annotations.is_empty() {
            measurement.annotations = annotations;
        }

        telemetry_sender.send(measurement.clone()).map_err(|e| {
            println!(
                "Could not send telemetry point: {}\n\n {:#?}",
                e, measurement
            );
            OcypodeError::TelemetryBroadcastError {
                source: Box::new(e),
            }
        })?;
        if let Some(ref writer_sender) = telemetry_writer_sender {
            writer_sender.send(measurement.clone()).map_err(|e| {
                println!(
                    "Could not send telemetry point: {}\n\n {:#?}",
                    e, measurement
                );
                OcypodeError::TelemetryBroadcastError {
                    source: Box::new(e),
                }
            })?;
        }
    }
}
