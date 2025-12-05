use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::mpsc::Receiver,
};

use log::warn;

use crate::{OcypodeError, telemetry::TelemetryOutput};

#[cfg(test)]
use std::io::BufRead;

/// Writes telemetry data to a file in JSON Lines format.
///
/// # File Format
///
/// The telemetry file uses JSON Lines format (also known as newline-delimited JSON),
/// where each line is a valid JSON object representing a single telemetry event.
///
/// ## TelemetryOutput Variants
///
/// Each line in the file is one of two types:
///
/// ### DataPoint
/// Contains telemetry data from a single moment in time using the `TelemetryData` structure.
/// All fields use explicit unit suffixes for clarity (e.g., `_rad`, `_mps`, `_deg`).
///
/// Key fields include:
/// - `point_no`: Sequential point number
/// - `timestamp_ms`: Unix timestamp in milliseconds
/// - `game_source`: The source game (e.g., "IRacing" or "ACC")
/// - Vehicle state: `gear`, `speed_mps`, `engine_rpm`, `max_engine_rpm`, `shift_point_rpm`
/// - Inputs: `throttle`, `brake`, `clutch`, `steering_angle_rad`, `steering_pct`
/// - Position data: `lap_distance_m`, `lap_distance_pct`, `lap_number`
/// - Timing: `last_lap_time_s`, `best_lap_time_s`
/// - Flags: `is_pit_limiter_engaged`, `is_in_pit_lane`, `is_abs_active`
/// - GPS (iRacing only): `latitude_deg`, `longitude_deg`
/// - Acceleration: `lateral_accel_mps2`, `longitudinal_accel_mps2`
/// - Orientation: `pitch_rad`, `roll_rad`, `yaw_rad`
/// - Rates (iRacing only): `pitch_rate_rps`, `roll_rate_rps`, `yaw_rate_rps`
/// - Tire data: `lf_tire_info`, `rf_tire_info`, `lr_tire_info`, `rr_tire_info`
/// - `annotations`: Array of analyzer-generated annotations (slip, wheelspin, etc.)
///
/// Example:
/// ```json
/// {"DataPoint":{"point_no":1,"timestamp_ms":1234567890,"game_source":"IRacing","gear":3,"speed_mps":45.2,...}}
/// ```
///
/// ### SessionChange
/// Contains session metadata when a new session is detected:
/// - `track_name`: Name of the track
/// - `track_configuration`: Track configuration/layout
/// - `max_steering_angle`: Maximum steering angle in degrees
/// - `track_length`: Track length as a string
/// - `game_source`: The source game (e.g., "IRacing" or "ACC")
/// - Game-specific fields (optional): series ID, session ID, season ID, etc.
///
/// Example:
/// ```json
/// {"SessionChange":{"track_name":"Laguna Seca","track_configuration":"Full Course","game_source":"IRacing",...}}
/// ```
///
/// ## Game Source Field
///
/// The `game_source` field is included in both DataPoint and SessionChange variants,
/// allowing analysis tools to identify which racing simulation the data came from.
/// This enables game-specific processing and ensures compatibility when loading
/// telemetry files for analysis.
///
/// ## Compatibility
///
/// Files written with this format are not compatible with older versions of Ocypode
/// that used the legacy TelemetryPoint format. When loading files, the application
/// will detect the format and provide a clear error message if an incompatible
/// legacy file is encountered.
pub fn write_telemetry(
    file: &PathBuf,
    telemetry_receiver: Receiver<TelemetryOutput>,
) -> Result<(), OcypodeError> {
    let telemetry_file = File::create(file).map_err(|e| OcypodeError::WriterError { source: e })?;
    let mut telemetry_file_writer = BufWriter::new(telemetry_file);
    
    for point in &telemetry_receiver {
        // Serialize TelemetryOutput to JSON
        // This includes TelemetryData (with game_source) for DataPoint
        // and SessionInfo (with game_source) for SessionChange
        let json_line = serde_json::to_string(&point)
            .map_err(|e| {
                warn!("Error serializing telemetry point: {}", e);
                e
            });
        
        match json_line {
            Ok(json) => {
                if let Err(e) = writeln!(telemetry_file_writer, "{}", json) {
                    warn!("Error while writing telemetry point to output file: {}", e);
                }
            }
            Err(e) => {
                warn!("Skipping telemetry point due to serialization error: {}", e);
            }
        }
    }
    
    telemetry_file_writer
        .flush()
        .map_err(|e| OcypodeError::WriterError { source: e })?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{TelemetryData, SessionInfo, GameSource};
    use std::sync::mpsc;
    use std::io::BufReader;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_telemetry_includes_game_source_in_datapoint() {
        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();

        // Create a channel and send some test data
        let (tx, rx) = mpsc::channel();

        // Create a TelemetryData with IRacing game source
        let mut telemetry = TelemetryData::default();
        telemetry.game_source = GameSource::IRacing;
        telemetry.point_no = 42;
        telemetry.gear = Some(3);
        telemetry.speed_mps = Some(45.5);

        // Send the data point
        tx.send(TelemetryOutput::DataPoint(telemetry.clone())).unwrap();
        drop(tx); // Close the channel so write_telemetry can finish

        // Write telemetry to file
        write_telemetry(&file_path, rx).unwrap();

        // Read the file and verify game_source is present
        let file = File::open(&file_path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

        assert_eq!(lines.len(), 1);
        
        // Parse the JSON and verify game_source is present
        let json_value: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        
        // Check that it's a DataPoint variant
        assert!(json_value.get("DataPoint").is_some());
        
        // Check that game_source is present and correct
        let data_point = json_value.get("DataPoint").unwrap();
        assert_eq!(data_point.get("game_source").unwrap(), "IRacing");
        assert_eq!(data_point.get("point_no").unwrap(), 42);
        assert_eq!(data_point.get("gear").unwrap(), 3);
    }

    #[test]
    fn test_write_telemetry_includes_game_source_in_session_change() {
        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();

        // Create a channel and send some test data
        let (tx, rx) = mpsc::channel();

        // Create a SessionInfo with ACC game source
        let mut session_info = SessionInfo::default();
        session_info.game_source = GameSource::ACC;
        session_info.track_name = "Monza".to_string();
        session_info.track_configuration = "Full Course".to_string();

        // Send the session change
        tx.send(TelemetryOutput::SessionChange(session_info)).unwrap();
        drop(tx); // Close the channel so write_telemetry can finish

        // Write telemetry to file
        write_telemetry(&file_path, rx).unwrap();

        // Read the file and verify game_source is present
        let file = File::open(&file_path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

        assert_eq!(lines.len(), 1);
        
        // Parse the JSON and verify game_source is present
        let json_value: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        
        // Check that it's a SessionChange variant
        assert!(json_value.get("SessionChange").is_some());
        
        // Check that game_source is present and correct
        let session_change = json_value.get("SessionChange").unwrap();
        assert_eq!(session_change.get("game_source").unwrap(), "ACC");
        assert_eq!(session_change.get("track_name").unwrap(), "Monza");
    }

    #[test]
    fn test_write_telemetry_handles_multiple_entries() {
        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();

        // Create a channel and send multiple entries
        let (tx, rx) = mpsc::channel();

        // Send a session change
        let mut session_info = SessionInfo::default();
        session_info.game_source = GameSource::IRacing;
        session_info.track_name = "Laguna Seca".to_string();
        tx.send(TelemetryOutput::SessionChange(session_info)).unwrap();

        // Send multiple data points
        for i in 0..5 {
            let mut telemetry = TelemetryData::default();
            telemetry.game_source = GameSource::IRacing;
            telemetry.point_no = i;
            tx.send(TelemetryOutput::DataPoint(telemetry)).unwrap();
        }
        drop(tx);

        // Write telemetry to file
        write_telemetry(&file_path, rx).unwrap();

        // Read the file and verify all entries are present
        let file = File::open(&file_path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

        assert_eq!(lines.len(), 6); // 1 session change + 5 data points

        // Verify first line is SessionChange
        let first_json: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert!(first_json.get("SessionChange").is_some());

        // Verify remaining lines are DataPoints with correct point_no
        for (idx, line) in lines.iter().skip(1).enumerate() {
            let json: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(json.get("DataPoint").is_some());
            let data_point = json.get("DataPoint").unwrap();
            assert_eq!(data_point.get("point_no").unwrap(), idx);
            assert_eq!(data_point.get("game_source").unwrap(), "IRacing");
        }
    }
}
