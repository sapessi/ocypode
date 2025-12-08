// Error types for ocypode

use crate::telemetry::TelemetryOutput;
use snafu::Snafu;
use std::{io, sync::mpsc::SendError};

#[derive(Debug, Snafu)]
pub enum OcypodeError {
    // Errors for the iRacing client
    #[snafu(display("Unable to find iRacing session"))]
    NoIRacingFile { source: io::Error },
    #[snafu(display("Timeout waiting for iRacing session"))]
    IRacingConnectionTimeout,

    // Errors for the ACC client
    #[snafu(display("Timeout waiting for ACC session"))]
    #[allow(dead_code)]
    ACCConnectionTimeout,

    // Errors while reading and broadcasting telemetry data
    #[snafu(display("Missing iRacing client, session not initialized"))]
    MissingIRacingSession,
    #[snafu(display("Telemetry point producer error"))]
    TelemetryProducerError { description: String },
    #[snafu(display("Error broadcasting telemetry data point"))]
    TelemetryBroadcastError {
        source: Box<SendError<TelemetryOutput>>,
    },

    // Errors for the telemetry writer
    #[snafu(display("Error writing telemetry file"))]
    WriterError { source: io::Error },

    // Config management errors
    #[snafu(display("Could not find application data directory to save config file"))]
    NoConfigDir,
    #[snafu(display("Error writing config file"))]
    ConfigIOError { source: io::Error },
    #[snafu(display("Error serializing config file"))]
    ConfigSerializeError { source: serde_json::Error },

    // UI errors
    #[snafu(display("Invalid telemetry file: {path}"))]
    InvalidTelemetryFile { path: String },
    #[snafu(display("Error loading telemetry file"))]
    TelemetryLoaderError { source: io::Error },
    #[snafu(display(
        "Legacy telemetry file format detected. This file was created with an older version of Ocypode and is not compatible with the current version. Please re-record your session with the current version."
    ))]
    LegacyTelemetryFormat,
}

impl From<SendError<TelemetryOutput>> for OcypodeError {
    fn from(value: SendError<TelemetryOutput>) -> Self {
        OcypodeError::TelemetryBroadcastError {
            source: Box::new(value),
        }
    }
}
