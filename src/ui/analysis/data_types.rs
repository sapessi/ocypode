use crate::telemetry::{SessionInfo, TelemetryData};

#[derive(Default, Clone, Debug)]
pub struct TelemetryFile {
    pub sessions: Vec<Session>,
}

#[derive(Default, Clone, Debug)]
pub struct Lap {
    pub telemetry: Vec<TelemetryData>,
}

#[derive(Default, Clone, Debug)]
pub struct Session {
    pub info: SessionInfo,
    pub laps: Vec<Lap>,
}

#[derive(Clone)]
pub enum UiState {
    Loading,
    Error { message: String },
    Display { session: Session },
}
