use std::{
    collections::HashMap,
    sync::mpsc::Sender,
    thread,
    time::{Duration, SystemTime},
};

use crate::OcypodeError;

use super::{
    producer::{TelemetryProducer, CONN_RETRY_MAX_WAIT_S},
    short_shifting_analyzer::ShortShiftingAnalyzer,
    slip_analyzer::SlipAnalyzer,
    trailbrake_steering_analyzer::{
        TrailbrakeSteeringAnalyzer, MAX_TRAILBRAKING_STEERING_ANGLE, MIN_TRAILBRAKING_PCT,
    },
    wheelspin_analyzer::WheelspinAnalyzer,
    TelemetryAnalyzer, TelemetryAnnotation, TelemetryOutput,
};

const REFRESH_RATE_MS: u64 = 100;
const MIN_WHEELSPIN_POINTS: usize = 100;
const SESSION_UPDATE_TIME_MS: u128 = 2000;

pub fn collect_telemetry(
    mut producer: impl TelemetryProducer,
    telemetry_sender: Sender<TelemetryOutput>,
    telemetry_writer_sender: Option<Sender<TelemetryOutput>>,
) -> Result<(), OcypodeError> {
    producer.start()?;

    wait_for_session(&mut producer)?;

    let mut analyzers: Vec<Box<dyn TelemetryAnalyzer>> = vec![
        Box::new(WheelspinAnalyzer::<MIN_WHEELSPIN_POINTS>::new()),
        Box::new(TrailbrakeSteeringAnalyzer::new(
            MAX_TRAILBRAKING_STEERING_ANGLE,
            MIN_TRAILBRAKING_PCT,
        )),
        Box::new(ShortShiftingAnalyzer::default()),
        Box::new(SlipAnalyzer::default()),
    ];

    // if we cannot fetch session info at this point something has gone really wrong.
    // I'll just let it fail.
    let mut last_session_info_check_time = SystemTime::now();
    let mut last_session_info = producer.session_info().unwrap();
    telemetry_sender.send(TelemetryOutput::SessionChange(last_session_info.clone()))?;
    if let Some(ref writer_sender) = telemetry_writer_sender {
        writer_sender.send(TelemetryOutput::SessionChange(last_session_info.clone()))?;
    }

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
                if session_info.we_session_id != last_session_info.we_session_id
                    || session_info.we_sub_session_id != last_session_info.we_sub_session_id
                    || session_info.track_name != last_session_info.track_name
                {
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

        let mut measurement = producer.telemetry()?;
        let mut annotations: HashMap<String, TelemetryAnnotation> = HashMap::new();
        for analyzer in analyzers.iter_mut() {
            annotations.extend(analyzer.analyze(&measurement, &last_session_info));
        }
        if !annotations.is_empty() {
            measurement.annotations = annotations;
        }

        telemetry_sender.send(TelemetryOutput::DataPoint(measurement.clone()))?;
        if let Some(ref writer_sender) = telemetry_writer_sender {
            writer_sender.send(TelemetryOutput::DataPoint(measurement.clone()))?;
        }
    }
}

fn wait_for_session(producer: &mut impl TelemetryProducer) -> Result<(), OcypodeError> {
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
    Ok(())
}
