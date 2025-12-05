use std::{
    thread,
    time::{Duration, SystemTime},
};

use log::{error, warn};
use simetry::Moment;

use crate::OcypodeError;

use super::{GameSource, MockMoment, SerializableTelemetry, SessionInfo};

const CONN_RETRY_WAIT_MS: u64 = 200;
const MAX_STEERING_ANGLE_DEFAULT: f32 = std::f32::consts::PI;
pub(crate) const CONN_RETRY_MAX_WAIT_S: u64 = 600;

/// A trait for producing telemetry data from racing simulation games.
/// 
/// This trait abstracts the telemetry data source, allowing the application to work with
/// multiple racing games through the simetry library's unified interface. Implementations
/// can connect to live game sessions or provide mock data for testing and offline analysis.
/// 
/// # Type System
/// 
/// The trait returns `Box<dyn Moment>` from the `telemetry()` method, where `Moment` is
/// simetry's trait for accessing telemetry data. This allows different game-specific
/// implementations to return their own telemetry types while maintaining a common interface.
/// 
/// # Lifecycle
/// 
/// 1. Call `start()` to initialize the connection to the game or data source
/// 2. Call `session_info()` to retrieve session metadata (track, configuration, etc.)
/// 3. Call `telemetry()` repeatedly to get telemetry data points
/// 4. Use `game_source()` to identify which game the data is coming from
pub trait TelemetryProducer {
    /// Initialize the telemetry producer and establish connection to the data source.
    /// 
    /// For live game producers, this typically involves connecting to the game's shared
    /// memory or network interface. May retry connection attempts with timeout.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the connection cannot be established within the timeout period.
    fn start(&mut self) -> Result<(), OcypodeError>;
    
    /// Retrieve session information including track name, configuration, and identifiers.
    /// 
    /// Session information is typically static for the duration of a session but may change
    /// when the user starts a new session or changes tracks.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the producer is not started or if session info cannot be retrieved.
    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError>;
    
    /// Get the next telemetry data point from the game.
    /// 
    /// Returns a boxed `Moment` trait object containing the current telemetry state.
    /// The returned data can be converted to `SerializableTelemetry` for storage or
    /// passed directly to analyzers for processing.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the producer is not started, if telemetry data cannot be
    /// retrieved, or if the data source is exhausted (for mock producers).
    fn telemetry(&mut self) -> Result<Box<dyn Moment>, OcypodeError>;
    
    /// Identify which racing game this producer is connected to.
    /// 
    /// This allows downstream components to handle game-specific differences in
    /// telemetry data availability and format.
    fn game_source(&self) -> GameSource;
}

#[cfg(windows)]
pub(crate) struct IRacingTelemetryProducer {
    client: Option<simetry::iracing::Client>,
    retry_wait_ms: u64,
    retry_timeout_s: u64,
    point_no: usize,
}

#[cfg(windows)]
impl Default for IRacingTelemetryProducer {
    fn default() -> Self {
        IRacingTelemetryProducer::new(CONN_RETRY_WAIT_MS, CONN_RETRY_MAX_WAIT_S)
    }
}

#[cfg(windows)]
impl IRacingTelemetryProducer {
    pub fn new(retry_wait_ms: u64, retry_timeout_s: u64) -> Self {
        Self {
            client: None,
            retry_wait_ms,
            retry_timeout_s,
            point_no: 0,
        }
    }
}

#[cfg(windows)]
impl TelemetryProducer for IRacingTelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError> {
        let start_time = SystemTime::now();
        let mut conn_result: Result<simetry::iracing::Client, simetry::iracing::Error> = 
            simetry::iracing::Client::connect();
        
        while conn_result.is_err() {
            if SystemTime::now()
                .duration_since(start_time)
                .unwrap()
                .as_secs()
                >= self.retry_timeout_s
            {
                error!(
                    "Could not create iRacing connection after {} seconds",
                    self.retry_timeout_s
                );
                return Err(OcypodeError::IRacingConnectionTimeout);
            }
            thread::sleep(Duration::from_millis(self.retry_wait_ms));
            conn_result = simetry::iracing::Client::connect();
        }
        
        self.client = Some(conn_result.map_err(|e| {
            error!("Failed to connect to iRacing: {:?}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Failed to connect to iRacing: {:?}", e),
            }
        })?);
        Ok(())
    }

    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The iRacing connection is not initialized, call start() first."
                    .to_string(),
            });
        }
        
        let client = self.client.as_mut().expect("Missing iRacing connection");
        
        // Get the current state from simetry
        let state = client.sample().map_err(|e| {
            warn!("Could not retrieve telemetry state: {:?}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Could not retrieve telemetry state: {:?}", e),
            }
        })?;
        
        // Extract session info from the state
        // Note: simetry's iRacing implementation provides session data through the state
        let track_name = state.session_info()
            .and_then(|info| info.weekend_info.track_display_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        
        let track_config = state.session_info()
            .and_then(|info| info.weekend_info.track_config_name.clone())
            .unwrap_or_else(|| "".to_string());
        
        let track_length = state.session_info()
            .and_then(|info| info.weekend_info.track_length.clone())
            .unwrap_or_else(|| "0.0 km".to_string());
        
        // Extract iRacing-specific session IDs
        let session_info = state.session_info();
        let we_series_id = session_info.and_then(|info| Some(info.weekend_info.series_id));
        let we_session_id = session_info.and_then(|info| Some(info.weekend_info.session_id));
        let we_season_id = session_info.and_then(|info| Some(info.weekend_info.season_id));
        let we_sub_session_id = session_info.and_then(|info| Some(info.weekend_info.sub_session_id));
        let we_league_id = session_info.and_then(|info| Some(info.weekend_info.league_id));
        
        // Get max steering angle from telemetry data
        // This is a telemetry field, not session info
        let max_steering_angle = state.vehicle_max_steer_angle()
            .map(|angle| angle.get::<uom::si::angle::radian>() as f32)
            .unwrap_or(MAX_STEERING_ANGLE_DEFAULT);

        Ok(SessionInfo {
            track_name,
            track_configuration: track_config,
            max_steering_angle,
            track_length,
            game_source: GameSource::IRacing,
            we_series_id,
            we_session_id,
            we_season_id,
            we_sub_session_id,
            we_league_id,
        })
    }

    fn telemetry(&mut self) -> Result<Box<dyn Moment>, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The iRacing connection is not initialized, call start() first."
                    .to_string(),
            });
        }
        
        let client = self.client.as_mut().ok_or(OcypodeError::MissingIRacingSession)?;
        
        let state = client.sample().map_err(|e| {
            error!("Could not retrieve telemetry: {:?}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Could not retrieve telemetry: {:?}", e),
            }
        })?;
        
        if self.point_no == usize::MAX {
            self.point_no = 0;
        }
        self.point_no += 1;
        
        Ok(Box::new(state))
    }
    
    fn game_source(&self) -> GameSource {
        GameSource::IRacing
    }
}

#[cfg(windows)]
pub(crate) struct ACCTelemetryProducer {
    client: Option<simetry::assetto_corsa_competizione::Client>,
    retry_wait_ms: u64,
    retry_timeout_s: u64,
    point_no: usize,
}

#[cfg(windows)]
impl Default for ACCTelemetryProducer {
    fn default() -> Self {
        ACCTelemetryProducer::new(CONN_RETRY_WAIT_MS, CONN_RETRY_MAX_WAIT_S)
    }
}

#[cfg(windows)]
impl ACCTelemetryProducer {
    pub fn new(retry_wait_ms: u64, retry_timeout_s: u64) -> Self {
        Self {
            client: None,
            retry_wait_ms,
            retry_timeout_s,
            point_no: 0,
        }
    }
}

#[cfg(windows)]
impl TelemetryProducer for ACCTelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError> {
        let start_time = SystemTime::now();
        let mut conn_result: Result<simetry::assetto_corsa_competizione::Client, simetry::assetto_corsa_competizione::Error> = 
            simetry::assetto_corsa_competizione::Client::connect();
        
        while conn_result.is_err() {
            if SystemTime::now()
                .duration_since(start_time)
                .unwrap()
                .as_secs()
                >= self.retry_timeout_s
            {
                error!(
                    "Could not create ACC connection after {} seconds",
                    self.retry_timeout_s
                );
                return Err(OcypodeError::ACCConnectionTimeout);
            }
            thread::sleep(Duration::from_millis(self.retry_wait_ms));
            conn_result = simetry::assetto_corsa_competizione::Client::connect();
        }
        
        self.client = Some(conn_result.map_err(|e| {
            error!("Failed to connect to ACC: {:?}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Failed to connect to ACC: {:?}", e),
            }
        })?);
        Ok(())
    }

    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The ACC connection is not initialized, call start() first."
                    .to_string(),
            });
        }
        
        let client = self.client.as_mut().expect("Missing ACC connection");
        
        // Get the current state from simetry
        let state = client.sample().map_err(|e| {
            warn!("Could not retrieve telemetry state: {:?}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Could not retrieve telemetry state: {:?}", e),
            }
        })?;
        
        // Extract session info from the state
        // ACC provides track name through simetry
        let track_name = state.session_info()
            .and_then(|info| info.track.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        
        // ACC doesn't provide separate track configuration in the same way as iRacing
        let track_config = String::new();
        
        // ACC doesn't provide track length in the same format
        let track_length = String::new();
        
        // Get max steering angle from telemetry data
        let max_steering_angle = state.vehicle_max_steer_angle()
            .map(|angle| angle.get::<uom::si::angle::radian>() as f32)
            .unwrap_or(MAX_STEERING_ANGLE_DEFAULT);

        // ACC doesn't have iRacing-specific session IDs, so all are None
        Ok(SessionInfo {
            track_name,
            track_configuration: track_config,
            max_steering_angle,
            track_length,
            game_source: GameSource::ACC,
            we_series_id: None,
            we_session_id: None,
            we_season_id: None,
            we_sub_session_id: None,
            we_league_id: None,
        })
    }

    fn telemetry(&mut self) -> Result<Box<dyn Moment>, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The ACC connection is not initialized, call start() first."
                    .to_string(),
            });
        }
        
        let client = self.client.as_mut().ok_or(OcypodeError::TelemetryProducerError {
            description: "Missing ACC session".to_string(),
        })?;
        
        let state = client.sample().map_err(|e| {
            error!("Could not retrieve telemetry: {:?}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Could not retrieve telemetry: {:?}", e),
            }
        })?;
        
        if self.point_no == usize::MAX {
            self.point_no = 0;
        }
        self.point_no += 1;
        
        Ok(Box::new(state))
    }
    
    fn game_source(&self) -> GameSource {
        GameSource::ACC
    }
}



/// A mock telemetry producer for testing and offline analysis.
/// 
/// MockTelemetryProducer allows the application to work with pre-recorded telemetry data
/// or programmatically generated test data. It implements the TelemetryProducer trait
/// using SerializableTelemetry data and returns MockMoment instances that implement
/// the simetry Moment trait.
/// 
/// This enables:
/// - Unit testing of telemetry processing logic without requiring a running game
/// - Offline analysis of previously recorded telemetry sessions
/// - Reproducible test scenarios for analyzer validation
pub(crate) struct MockTelemetryProducer {
    cur_tick: usize,
    points: Vec<SerializableTelemetry>,
    pub track_name: String,
    pub max_steering_angle: f32,
    pub game_source: GameSource,
}

impl Default for MockTelemetryProducer {
    fn default() -> Self {
        Self {
            cur_tick: 0,
            points: Vec::new(),
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
            game_source: GameSource::IRacing,
        }
    }
}

#[allow(dead_code)]
impl MockTelemetryProducer {
    /// Create a MockTelemetryProducer from a vector of SerializableTelemetry points.
    /// 
    /// # Arguments
    /// 
    /// * `points` - A vector of SerializableTelemetry data points to replay
    /// 
    /// # Returns
    /// 
    /// A new MockTelemetryProducer initialized with the provided data points
    pub fn from_points(points: Vec<SerializableTelemetry>) -> Self {
        // Infer game source from the first point if available
        let game_source = points.first()
            .map(|p| p.game_source)
            .unwrap_or(GameSource::IRacing);
        
        Self {
            cur_tick: 0,
            points,
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
            game_source,
        }
    }

    /// Load telemetry data from a JSON file.
    /// 
    /// The file should contain a JSON array of SerializableTelemetry objects,
    /// typically created by the telemetry writer during a live session.
    /// 
    /// # Arguments
    /// 
    /// * `file` - Path to the JSON file containing telemetry data
    /// 
    /// # Returns
    /// 
    /// A Result containing the MockTelemetryProducer or an error if the file
    /// cannot be read or parsed
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The file cannot be opened
    /// - The file contains invalid JSON
    /// - The JSON does not match the SerializableTelemetry format
    pub fn from_file(file: &str) -> Result<Self, OcypodeError> {
        let file =
            std::fs::File::open(file).map_err(|e| OcypodeError::NoIRacingFile { source: e })?;
        let reader = std::io::BufReader::new(file);
        let points: Vec<SerializableTelemetry> = serde_json::from_reader(reader).map_err(|e| {
            error!("Could not load JSON file: {}", e);
            OcypodeError::TelemetryProducerError {
                description: format!("Could not load JSON file: {}", e),
            }
        })?;

        // Infer game source from the first point if available
        let game_source = points.first()
            .map(|p| p.game_source)
            .unwrap_or(GameSource::IRacing);

        Ok(Self {
            cur_tick: 0,
            points,
            track_name: "Unknown".to_string(),
            max_steering_angle: 0.,
            game_source,
        })
    }
}

impl TelemetryProducer for MockTelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError> {
        // Mock producer doesn't need to connect to anything
        Ok(())
    }

    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError> {
        Ok(SessionInfo {
            track_name: self.track_name.clone(),
            track_configuration: String::new(),
            max_steering_angle: self.max_steering_angle,
            track_length: "1.5".to_string(),
            game_source: self.game_source,
            we_series_id: Some(0),
            we_session_id: Some(0),
            we_season_id: Some(0),
            we_sub_session_id: Some(0),
            we_league_id: Some(0),
        })
    }

    fn telemetry(&mut self) -> Result<Box<dyn Moment>, OcypodeError> {
        if self.cur_tick >= self.points.len() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "End of points vec".to_string(),
            });
        }
        
        let point = self.points[self.cur_tick].clone();
        self.cur_tick += 1;
        
        Ok(Box::new(MockMoment::new(point)))
    }
    
    fn game_source(&self) -> GameSource {
        self.game_source
    }
}
