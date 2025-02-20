use std::{collections::HashMap, sync::mpsc::Sender, thread, time::Duration};

use crate::telemetry::TelemetryPoint;

use crate::OcypodeError;

use super::{
    producer::TelemetryProducer, trailbrake_steering_analyzer::TrailbrakeSteeringAnalyzer,
    wheelspin_analyzer::WheelspinAnalyzer, TelemetryAnalyzer, TelemetryAnnotation,
};

const REFRESH_RATE_MS: u64 = 100;
const MIN_WHEELSPIN_POINTS: usize = 500;

const MIN_TRAILBRAKING_PCT: f32 = 0.2;
const MAX_TRAILBRAKING_STEERING_ANGLE: f32 = 0.1;

pub fn collect_telemetry(
    mut producer: impl TelemetryProducer,
    telemetry_sender: Sender<TelemetryPoint>,
    telemetry_writer_sender: Option<Sender<TelemetryPoint>>,
) -> Result<(), OcypodeError> {
    producer.start()?;

    let mut analyzers: Vec<Box<dyn TelemetryAnalyzer>> = vec![
        Box::new(WheelspinAnalyzer::<MIN_WHEELSPIN_POINTS>::new()),
        Box::new(TrailbrakeSteeringAnalyzer::new(
            MAX_TRAILBRAKING_STEERING_ANGLE,
            MIN_TRAILBRAKING_PCT,
        )),
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
