use std::{
    sync::mpsc::Sender,
    thread,
    time::{Duration, SystemTime},
};

use crate::OcypodeError;

use super::{
    TelemetryAnalyzer, TelemetryAnnotation, TelemetryOutput,
    bottoming_out_analyzer::BottomingOutAnalyzer,
    brake_lock_analyzer::BrakeLockAnalyzer,
    entry_oversteer_analyzer::EntryOversteerAnalyzer,
    mid_corner_analyzer::MidCornerAnalyzer,
    producer::{CONN_RETRY_MAX_WAIT_S, TelemetryProducer},
    scrub_analyzer::ScrubAnalyzer,
    short_shifting_analyzer::ShortShiftingAnalyzer,
    slip_analyzer::SlipAnalyzer,
    tire_temperature_analyzer::TireTemperatureAnalyzer,
    trailbrake_steering_analyzer::{
        MAX_TRAILBRAKING_STEERING_ANGLE, MIN_TRAILBRAKING_PCT, TrailbrakeSteeringAnalyzer,
    },
    wheelspin_analyzer::WheelspinAnalyzer,
};

const REFRESH_RATE_MS: u64 = 100;
const MIN_WHEELSPIN_POINTS: usize = 100;
const SESSION_UPDATE_TIME_MS: u128 = 2000;

// Configuration for new analyzers
const ENTRY_OVERSTEER_WINDOW_SIZE: usize = 100;
const ENTRY_OVERSTEER_MIN_POINTS: usize = 50;
const MID_CORNER_WINDOW_SIZE: usize = 100;
const MID_CORNER_MIN_POINTS: usize = 50;

pub fn collect_telemetry(
    mut producer: impl TelemetryProducer,
    telemetry_sender: Sender<TelemetryOutput>,
    telemetry_writer_sender: Option<Sender<TelemetryOutput>>,
) -> Result<(), OcypodeError> {
    use log::{debug, info};

    info!("Telemetry collector: Starting producer...");
    producer.start()?;
    info!("Telemetry collector: Producer started, waiting for active session...");

    wait_for_session(&mut producer)?;
    info!("Telemetry collector: Active session detected, beginning data collection...");

    let mut analyzers: Vec<Box<dyn TelemetryAnalyzer>> = vec![
        // Existing analyzers
        Box::new(WheelspinAnalyzer::<MIN_WHEELSPIN_POINTS>::new()),
        Box::new(TrailbrakeSteeringAnalyzer::new(
            MAX_TRAILBRAKING_STEERING_ANGLE,
            MIN_TRAILBRAKING_PCT,
        )),
        Box::new(ShortShiftingAnalyzer::default()),
        Box::new(SlipAnalyzer::default()),
        Box::new(ScrubAnalyzer::<100>::new(100)), // TODO: The maximum number of points should be dynamic based on the length of the track
        // New analyzers for Setup Assistant
        Box::new(EntryOversteerAnalyzer::<ENTRY_OVERSTEER_WINDOW_SIZE>::new(
            ENTRY_OVERSTEER_MIN_POINTS,
        )),
        Box::new(MidCornerAnalyzer::<MID_CORNER_WINDOW_SIZE>::new(
            MID_CORNER_MIN_POINTS,
        )),
        Box::new(BrakeLockAnalyzer::new()),
        Box::new(TireTemperatureAnalyzer::new()),
        Box::new(BottomingOutAnalyzer::new()),
    ];

    // if we cannot fetch session info at this point something has gone really wrong.
    // I'll just let it fail.
    let mut last_session_info_check_time = SystemTime::now();
    let mut last_session_info = producer.session_info().unwrap();

    info!(
        "Telemetry collector: Sending initial session info (track: {})",
        last_session_info.track_name
    );
    telemetry_sender.send(TelemetryOutput::SessionChange(last_session_info.clone()))?;
    if let Some(ref writer_sender) = telemetry_writer_sender {
        writer_sender.send(TelemetryOutput::SessionChange(last_session_info.clone()))?;
    }

    info!("Telemetry collector: Entering main collection loop...");
    let mut points_collected = 0;

    loop {
        thread::sleep(Duration::from_millis(REFRESH_RATE_MS));

        // check whether we need to update the session
        if SystemTime::now()
            .duration_since(last_session_info_check_time)
            .unwrap()
            .as_millis()
            >= SESSION_UPDATE_TIME_MS
        {
            if let Ok(session_info) = producer.session_info() {
                // Check for session changes - handle optional fields properly
                let session_changed = session_info.we_session_id != last_session_info.we_session_id
                    || session_info.we_sub_session_id != last_session_info.we_sub_session_id
                    || session_info.track_name != last_session_info.track_name;

                if session_changed {
                    last_session_info = session_info.clone();
                    telemetry_sender.send(TelemetryOutput::SessionChange(session_info.clone()))?;
                    if let Some(ref writer_sender) = telemetry_writer_sender {
                        writer_sender.send(TelemetryOutput::SessionChange(session_info.clone()))?;
                    }
                }
            } else {
                // we may be changing sessions... let's wait
                wait_for_session(&mut producer)?;
                continue;
            }
            last_session_info_check_time = SystemTime::now();
        }

        // Get telemetry as TelemetryData
        let mut telemetry_data = producer.telemetry()?;
        points_collected += 1;

        if points_collected == 1 {
            info!("Telemetry collector: First data point received!");
        } else if points_collected % 100 == 0 {
            debug!("Telemetry collector: {} points collected", points_collected);
        }

        // Run analyzers on the TelemetryData
        // Pre-allocate with capacity to avoid reallocations
        let mut annotations: Vec<TelemetryAnnotation> = Vec::with_capacity(10);
        for analyzer in analyzers.iter_mut() {
            annotations.append(&mut analyzer.analyze(&telemetry_data, &last_session_info));
        }

        // Add annotations to the telemetry data
        if !annotations.is_empty() {
            telemetry_data.annotations = annotations;
        }

        // Box the telemetry data once and clone the Box (cheaper than cloning the data)
        let boxed_data = Box::new(telemetry_data);
        telemetry_sender.send(TelemetryOutput::DataPoint(boxed_data.clone()))?;
        if let Some(ref writer_sender) = telemetry_writer_sender {
            writer_sender.send(TelemetryOutput::DataPoint(boxed_data))?;
        }
    }
}

fn wait_for_session(producer: &mut impl TelemetryProducer) -> Result<(), OcypodeError> {
    use log::{info, warn};

    // wait for a session to start
    let session_wait_start = SystemTime::now();
    let mut last_log_time = SystemTime::now();
    let mut retry_count = 0;

    loop {
        if producer.session_info().is_err() {
            retry_count += 1;
            thread::sleep(Duration::from_millis(REFRESH_RATE_MS));

            // Log every 5 seconds
            if SystemTime::now()
                .duration_since(last_log_time)
                .unwrap()
                .as_secs()
                >= 5
            {
                let elapsed = SystemTime::now()
                    .duration_since(session_wait_start)
                    .unwrap()
                    .as_secs();
                info!(
                    "Still waiting for active session... ({} seconds elapsed, {} retries)",
                    elapsed, retry_count
                );
                last_log_time = SystemTime::now();
            }
        } else {
            info!("Active session found after {} retries!", retry_count);
            break;
        }

        let elapsed_secs = SystemTime::now()
            .duration_since(session_wait_start)
            .unwrap()
            .as_secs();

        if elapsed_secs > CONN_RETRY_MAX_WAIT_S {
            warn!("Timeout waiting for session after {} seconds", elapsed_secs);
            return Err(OcypodeError::IRacingConnectionTimeout);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::producer::MockTelemetryProducer;
    use crate::telemetry::{GameSource, TelemetryData, TelemetryOutput};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::thread;

    #[test]
    fn test_collect_telemetry_with_writer() {
        let (telemetry_sender, telemetry_receiver): (
            Sender<TelemetryOutput>,
            Receiver<TelemetryOutput>,
        ) = mpsc::channel();
        let (writer_sender, writer_receiver): (Sender<TelemetryOutput>, Receiver<TelemetryOutput>) =
            mpsc::channel();

        let points = vec![
            TelemetryData {
                point_no: 0,
                timestamp_ms: 0,
                game_source: GameSource::IRacing,
                gear: Some(2),
                engine_rpm: Some(5000.0),
                shift_point_rpm: Some(5200.0),
                speed_mps: Some(50.0),
                throttle: Some(0.8),
                brake: Some(0.0),
                clutch: Some(0.0),
                ..Default::default()
            },
            TelemetryData {
                point_no: 1,
                timestamp_ms: 100,
                game_source: GameSource::IRacing,
                gear: Some(3),
                engine_rpm: Some(5100.0),
                shift_point_rpm: Some(5200.0),
                speed_mps: Some(55.0),
                throttle: Some(0.9),
                brake: Some(0.0),
                clutch: Some(0.0),
                ..Default::default()
            },
        ];

        let mut mock_producer = MockTelemetryProducer::from_points(points);
        mock_producer.track_name = "Test Track".to_string();
        mock_producer.max_steering_angle = 720.0;

        let handle = thread::spawn(move || {
            let _ = collect_telemetry(mock_producer, telemetry_sender, Some(writer_sender));
        });

        thread::sleep(Duration::from_millis(REFRESH_RATE_MS * 3));

        // Check if session change was sent
        let session_change = telemetry_receiver.recv().unwrap();
        if let TelemetryOutput::SessionChange(session_info) = session_change {
            assert_eq!(session_info.track_name, "Test Track");
        } else {
            panic!("Expected SessionChange");
        }

        // Check if telemetry data points were sent
        for _ in 0..2 {
            let data_point = telemetry_receiver.recv().unwrap();
            if let TelemetryOutput::DataPoint(measurement) = data_point {
                assert!(measurement.gear == Some(2) || measurement.gear == Some(3));
                assert!(measurement.gear == Some(2) || measurement.gear == Some(3));
            } else {
                panic!("Expected DataPoint");
            }
        }

        // Check if writer received the same data
        let session_change = writer_receiver.recv().unwrap();
        if let TelemetryOutput::SessionChange(session_info) = session_change {
            assert_eq!(session_info.track_name, "Test Track");
        } else {
            panic!("Expected SessionChange");
        }
        for _ in 0..2 {
            let writer_data_point = writer_receiver.recv().unwrap();
            if let TelemetryOutput::DataPoint(measurement) = writer_data_point {
                assert!(measurement.gear == Some(2) || measurement.gear == Some(3));
                assert!(measurement.gear == Some(2) || measurement.gear == Some(3));
            } else {
                panic!("Expected DataPoint: {:?}", writer_data_point);
            }
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_collect_telemetry_no_writer() {
        let (telemetry_sender, telemetry_receiver): (
            Sender<TelemetryOutput>,
            Receiver<TelemetryOutput>,
        ) = mpsc::channel();

        let points = vec![
            TelemetryData {
                point_no: 0,
                timestamp_ms: 0,
                game_source: GameSource::IRacing,
                gear: Some(2),
                engine_rpm: Some(5000.0),
                shift_point_rpm: Some(5200.0),
                speed_mps: Some(50.0),
                throttle: Some(0.8),
                brake: Some(0.0),
                clutch: Some(0.0),
                ..Default::default()
            },
            TelemetryData {
                point_no: 1,
                timestamp_ms: 100,
                game_source: GameSource::IRacing,
                gear: Some(3),
                engine_rpm: Some(5100.0),
                shift_point_rpm: Some(5200.0),
                speed_mps: Some(55.0),
                throttle: Some(0.9),
                brake: Some(0.0),
                clutch: Some(0.0),
                ..Default::default()
            },
        ];

        let mut mock_producer = MockTelemetryProducer::from_points(points);
        mock_producer.track_name = "Test Track".to_string();
        mock_producer.max_steering_angle = 720.0;

        let handle = thread::spawn(move || {
            let _ = collect_telemetry(mock_producer, telemetry_sender, None);
        });

        // Check if session change was sent
        let session_change = telemetry_receiver.recv().unwrap();
        if let TelemetryOutput::SessionChange(session_info) = session_change {
            assert_eq!(session_info.track_name, "Test Track");
        } else {
            panic!("Expected SessionChange");
        }

        // Check if telemetry data points were sent
        for _ in 0..2 {
            let data_point = telemetry_receiver.recv().unwrap();
            if let TelemetryOutput::DataPoint(measurement) = data_point {
                assert!(measurement.gear == Some(2) || measurement.gear == Some(3));
                assert!(measurement.gear == Some(2) || measurement.gear == Some(3));
            } else {
                panic!("Expected DataPoint");
            }
        }

        handle.join().unwrap();
    }
}
