use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use log::info;

use super::data_types::{Lap, Session, TelemetryFile};
use crate::{OcypodeError, telemetry::TelemetryOutput};

pub fn load_telemetry_jsonl(source_file: &PathBuf) -> Result<TelemetryFile, OcypodeError> {
    // Check if this is a legacy format file before attempting to deserialize
    if is_legacy_format(source_file) {
        return Err(OcypodeError::LegacyTelemetryFormat);
    }

    // TODO: Should probably load in a non-blocking way here
    let telemetry_lines = serde_jsonlines::json_lines(source_file)
        .map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?
        .collect::<Result<Vec<TelemetryOutput>, std::io::Error>>()
        .map_err(|e| {
            // If deserialization fails, check if it might be a legacy format
            // that we didn't catch in the initial check
            if is_legacy_format(source_file) {
                OcypodeError::LegacyTelemetryFormat
            } else {
                OcypodeError::TelemetryLoaderError { source: e }
            }
        })?;

    let mut telemetry_data = TelemetryFile::default();
    let mut cur_lap_no: u32 = 0;
    let mut cur_session = Session::default();
    let mut cur_lap = Lap::default();
    for line in telemetry_lines {
        match line {
            TelemetryOutput::DataPoint(telemetry_point) => {
                let lap_no = telemetry_point.lap_number.unwrap_or(0);
                if lap_no != cur_lap_no {
                    cur_session.laps.push(cur_lap.clone());
                    cur_lap = Lap::default();
                    cur_lap_no = lap_no;
                }
                cur_lap.telemetry.push(*telemetry_point);
            }
            TelemetryOutput::SessionChange(session_info) => {
                if !cur_lap.telemetry.is_empty() {
                    cur_session.laps.push(cur_lap);
                }
                // if we already have data points we are starting a new session
                if !cur_session.laps.is_empty() {
                    telemetry_data.sessions.push(cur_session.clone());
                    cur_session = Session::default();
                }
                cur_lap = Lap::default();
                cur_lap_no = 0;
                cur_session.info = session_info;
            }
        }
    }
    telemetry_data.sessions.push(cur_session);
    info!(
        "Loaded {:?}, found {} sessions with a total of {} laps",
        source_file,
        telemetry_data.sessions.len(),
        cur_lap_no
    );
    Ok(telemetry_data)
}

/// Detects if a telemetry file uses the legacy TelemetryPoint format
/// by attempting to parse the first line as a raw JSON value and checking
/// for the presence of legacy-specific fields.
pub fn is_legacy_format(source_file: &PathBuf) -> bool {
    // Try to read the first line of the file
    let file = match File::open(source_file) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut reader = BufReader::new(file);
    let mut first_line = String::new();

    if reader.read_line(&mut first_line).is_err() {
        return false;
    }

    // Try to parse as JSON
    let json_value: serde_json::Value = match serde_json::from_str(&first_line) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Check if it's a DataPoint variant
    if let Some(obj) = json_value.get("DataPoint") {
        // Legacy format has fields like "cur_gear", "cur_rpm", "cur_speed", "lap_dist"
        // New format has "gear", "engine_rpm", "speed_mps", "lap_distance"
        let has_legacy_fields = obj.get("cur_gear").is_some()
            || obj.get("cur_rpm").is_some()
            || obj.get("cur_speed").is_some()
            || obj.get("lap_dist").is_some()
            || obj.get("car_shift_ideal_rpm").is_some();

        let has_new_fields = obj.get("game_source").is_some();

        // If it has legacy fields and doesn't have new fields, it's legacy format
        return has_legacy_fields && !has_new_fields;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_legacy_format_detection() {
        // Create a temporary file with legacy format
        let mut legacy_file = NamedTempFile::new().unwrap();
        writeln!(
            legacy_file,
            r#"{{"DataPoint":{{"point_no":0,"point_epoch":1234567890,"lap_dist":100.0,"lap_dist_pct":0.5,"last_lap_time_s":90.0,"best_lap_time_s":88.0,"car_shift_ideal_rpm":7000.0,"cur_gear":3,"cur_rpm":5000.0,"cur_speed":50.0,"lap_no":1,"throttle":0.8,"brake":0.0,"steering":0.1,"steering_pct":0.05,"abs_active":false,"lat":0.0,"lon":0.0,"lat_accel":0.0,"lon_accel":0.0,"pitch":0.0,"pitch_rate":0.0,"roll":0.0,"roll_rate":0.0,"yaw":0.0,"yaw_rate":0.0,"lf_tire_info":null,"rf_tire_info":null,"lr_tire_info":null,"rr_tire_info":null,"annotations":[]}}}}"#
        ).unwrap();
        legacy_file.flush().unwrap();

        // Test that legacy format is detected
        assert!(is_legacy_format(&legacy_file.path().to_path_buf()));
    }

    #[test]
    fn test_new_format_not_detected_as_legacy() {
        // Create a temporary file with new format
        let mut new_file = NamedTempFile::new().unwrap();
        writeln!(
            new_file,
            r#"{{"DataPoint":{{"point_no":0,"timestamp_ms":1234567890,"game_source":"IRacing","gear":3,"speed_mps":50.0,"engine_rpm":5000.0,"max_engine_rpm":8000.0,"shift_point_rpm":7000.0,"throttle":0.8,"brake":0.0,"clutch":0.0,"steering":0.1,"steering_pct":0.05,"lap_distance":100.0,"lap_distance_pct":0.5,"lap_number":1,"last_lap_time_s":90.0,"best_lap_time_s":88.0,"is_pit_limiter_engaged":false,"is_in_pit_lane":false,"abs_active":false,"lat":0.0,"lon":0.0,"lat_accel":0.0,"lon_accel":0.0,"pitch":0.0,"pitch_rate":0.0,"roll":0.0,"roll_rate":0.0,"yaw":0.0,"yaw_rate":0.0,"lf_tire_info":null,"rf_tire_info":null,"lr_tire_info":null,"rr_tire_info":null,"annotations":[]}}}}"#
        ).unwrap();
        new_file.flush().unwrap();

        // Test that new format is NOT detected as legacy
        assert!(!is_legacy_format(&new_file.path().to_path_buf()));
    }

    #[test]
    fn test_load_legacy_format_returns_error() {
        // Create a temporary file with legacy format
        let mut legacy_file = NamedTempFile::new().unwrap();
        writeln!(
            legacy_file,
            r#"{{"DataPoint":{{"point_no":0,"point_epoch":1234567890,"lap_dist":100.0,"lap_dist_pct":0.5,"last_lap_time_s":90.0,"best_lap_time_s":88.0,"car_shift_ideal_rpm":7000.0,"cur_gear":3,"cur_rpm":5000.0,"cur_speed":50.0,"lap_no":1,"throttle":0.8,"brake":0.0,"steering":0.1,"steering_pct":0.05,"abs_active":false,"lat":0.0,"lon":0.0,"lat_accel":0.0,"lon_accel":0.0,"pitch":0.0,"pitch_rate":0.0,"roll":0.0,"roll_rate":0.0,"yaw":0.0,"yaw_rate":0.0,"lf_tire_info":null,"rf_tire_info":null,"lr_tire_info":null,"rr_tire_info":null,"annotations":[]}}}}"#
        ).unwrap();
        legacy_file.flush().unwrap();

        // Test that loading legacy format returns the correct error
        let result = load_telemetry_jsonl(&legacy_file.path().to_path_buf());
        assert!(result.is_err());
        match result {
            Err(OcypodeError::LegacyTelemetryFormat) => {
                // Expected error
            }
            _ => panic!("Expected LegacyTelemetryFormat error"),
        }
    }
}
