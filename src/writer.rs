use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::mpsc::Receiver,
};

use crate::{telemetry::TelemetryPoint, OcypodeError};

pub fn write_telemetry(
    file: &PathBuf,
    telemetry_receiver: Receiver<TelemetryPoint>,
) -> Result<(), OcypodeError> {
    let telemetry_file = File::create(file).map_err(|e| OcypodeError::WriterError { source: e })?;
    let mut telemetry_file_writer = BufWriter::new(telemetry_file);
    for point in &telemetry_receiver {
        if point.cur_gear == 0 {
            continue;
        }
        let _ = writeln!(
            telemetry_file_writer,
            "{}",
            serde_json::to_string(&point).unwrap()
        )
        .map_err(|e| {
            println!("Error while writing telemetry point to output file: {}", e);
        });
    }
    telemetry_file_writer
        .flush()
        .map_err(|e| OcypodeError::WriterError { source: e })?;
    Ok(())
}
