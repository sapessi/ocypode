use std::time::Duration;

use log::error;

use crate::OcypodeError;

use super::{GameSource, SessionInfo, TelemetryData, TelemetryOutput};

const CONN_RETRY_WAIT_MS: u64 = 200;
const MAX_STEERING_ANGLE_DEFAULT: f32 = std::f32::consts::PI;
pub(crate) const CONN_RETRY_MAX_WAIT_S: u64 = 600;

/// A trait for producing telemetry data from racing simulation games.
///
/// This trait abstracts the telemetry data source, allowing the application to work with
/// multiple racing games through a unified intermediate representation. Implementations
/// can connect to live game sessions or provide mock data for testing and offline analysis.
///
/// # Type System
///
/// The trait returns `TelemetryData` from the `telemetry()` method, which is a unified
/// intermediate representation that captures all possible telemetry data points from
/// supported games. This eliminates the need for unsafe downcasting and provides a
/// clean separation between game-specific implementations and analyzers.
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
    /// Returns a `TelemetryData` struct containing the current telemetry state in a
    /// unified intermediate representation. The data can be passed directly to analyzers
    /// for processing or serialized for storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the producer is not started, if telemetry data cannot be
    /// retrieved, or if the data source is exhausted (for mock producers).
    fn telemetry(&mut self) -> Result<TelemetryData, OcypodeError>;

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
        let retry_delay = Duration::from_millis(self.retry_wait_ms);

        let client = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(simetry::iracing::Client::connect(retry_delay));

        self.client = Some(client);
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

        // In simetry 0.2.3, use next_sim_state() to get the current state
        let state = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.next_sim_state())
            .ok_or(OcypodeError::TelemetryProducerError {
                description: "Could not retrieve iRacing state".to_string(),
            })?;

        // Extract session info from the YAML
        let session_info = state.session_info();
        let track_name = session_info["WeekendInfo"]["TrackDisplayName"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        let track_config = session_info["WeekendInfo"]["TrackConfigName"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let track_length = session_info["WeekendInfo"]["TrackLength"]
            .as_str()
            .unwrap_or("0.0 km")
            .to_string();

        // Extract iRacing-specific session IDs from the YAML session info
        let we_series_id = session_info["WeekendInfo"]["SeriesID"]
            .as_i64()
            .map(|v| v as i32);
        let we_session_id = session_info["WeekendInfo"]["SessionID"]
            .as_i64()
            .map(|v| v as i32);
        let we_season_id = session_info["WeekendInfo"]["SeasonID"]
            .as_i64()
            .map(|v| v as i32);
        let we_sub_session_id = session_info["WeekendInfo"]["SubSessionID"]
            .as_i64()
            .map(|v| v as i32);
        let we_league_id = session_info["WeekendInfo"]["LeagueID"]
            .as_i64()
            .map(|v| v as i32);

        // Use default max steering angle (simetry 0.2.3 doesn't expose this in the Moment trait)
        let max_steering_angle = MAX_STEERING_ANGLE_DEFAULT;

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

    fn telemetry(&mut self) -> Result<TelemetryData, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The iRacing connection is not initialized, call start() first."
                    .to_string(),
            });
        }

        let client = self
            .client
            .as_mut()
            .ok_or(OcypodeError::MissingIRacingSession)?;

        if self.point_no == usize::MAX {
            self.point_no = 0;
        }
        self.point_no += 1;

        // In simetry 0.2.3, use next_sim_state() to get the current state
        let state = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.next_sim_state())
            .ok_or(OcypodeError::TelemetryProducerError {
                description: "Could not retrieve iRacing telemetry".to_string(),
            })?;

        Ok(TelemetryData::from_iracing_state(&state, self.point_no))
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
        let retry_delay = Duration::from_millis(self.retry_wait_ms);

        let client = tokio::runtime::Runtime::new().unwrap().block_on(
            simetry::assetto_corsa_competizione::Client::connect(retry_delay),
        );

        self.client = Some(client);
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

        // In simetry 0.2.3, use next_sim_state() to get the current state
        let _state = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.next_sim_state())
            .ok_or(OcypodeError::TelemetryProducerError {
                description: "Could not retrieve ACC state".to_string(),
            })?;

        // For ACC, we need to extract track info from the static data
        // ACC doesn't provide as much session info as iRacing
        let track_name = "Unknown".to_string(); // ACC doesn't expose track name easily
        let track_config = String::new();
        let track_length = String::new();

        // Use default max steering angle (simetry 0.2.3 doesn't expose this in the Moment trait)
        let max_steering_angle = MAX_STEERING_ANGLE_DEFAULT;

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

    fn telemetry(&mut self) -> Result<TelemetryData, OcypodeError> {
        if self.client.is_none() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "The ACC connection is not initialized, call start() first."
                    .to_string(),
            });
        }

        let client = self
            .client
            .as_mut()
            .ok_or(OcypodeError::TelemetryProducerError {
                description: "Missing ACC session".to_string(),
            })?;

        if self.point_no == usize::MAX {
            self.point_no = 0;
        }
        self.point_no += 1;

        // In simetry 0.2.3, use next_sim_state() to get the current state
        let state = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.next_sim_state())
            .ok_or(OcypodeError::TelemetryProducerError {
                description: "Could not retrieve ACC telemetry".to_string(),
            })?;

        Ok(TelemetryData::from_acc_state(&state, self.point_no))
    }

    fn game_source(&self) -> GameSource {
        GameSource::ACC
    }
}

/// A mock telemetry producer for testing and offline analysis.
///
/// MockTelemetryProducer allows the application to work with pre-recorded telemetry data
/// or programmatically generated test data. It implements the TelemetryProducer trait
/// using TelemetryData directly.
///
/// This enables:
/// - Unit testing of telemetry processing logic without requiring a running game
/// - Offline analysis of previously recorded telemetry sessions
/// - Reproducible test scenarios for analyzer validation
pub(crate) struct MockTelemetryProducer {
    cur_tick: usize,
    points: Vec<TelemetryData>,
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
    /// Create a MockTelemetryProducer from a vector of TelemetryData points.
    ///
    /// # Arguments
    ///
    /// * `points` - A vector of TelemetryData data points to replay
    ///
    /// # Returns
    ///
    /// A new MockTelemetryProducer initialized with the provided data points
    pub fn from_points(points: Vec<TelemetryData>) -> Self {
        // Infer game source from the first point if available
        let game_source = points
            .first()
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

    /// Load telemetry data from a JSON Lines file.
    ///
    /// The file should contain TelemetryOutput objects in JSON Lines format,
    /// typically created by the telemetry writer during a live session.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the JSON Lines file containing telemetry data
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
    /// - The JSON does not match the TelemetryOutput format
    pub fn from_file(file: &str) -> Result<Self, OcypodeError> {
        use std::io::BufRead;

        let file =
            std::fs::File::open(file).map_err(|e| OcypodeError::NoIRacingFile { source: e })?;
        let reader = std::io::BufReader::new(file);

        let mut points = Vec::new();
        let mut track_name = "Unknown".to_string();
        let mut max_steering_angle = 0.0;

        for line in reader.lines() {
            let line = line.map_err(|e| OcypodeError::TelemetryProducerError {
                description: format!("Could not read line from file: {}", e),
            })?;

            // Parse as TelemetryOutput format
            let output: TelemetryOutput = serde_json::from_str(&line).map_err(|e| {
                error!("Could not parse JSON line: {}", e);
                OcypodeError::TelemetryProducerError {
                    description: format!("Could not parse JSON line: {}", e),
                }
            })?;

            match output {
                TelemetryOutput::DataPoint(telemetry) => {
                    points.push(telemetry);
                }
                TelemetryOutput::SessionChange(session) => {
                    track_name = session.track_name;
                    max_steering_angle = session.max_steering_angle;
                }
            }
        }

        // Infer game source from the first point if available
        let game_source = points
            .first()
            .map(|p| p.game_source)
            .unwrap_or(GameSource::IRacing);

        Ok(Self {
            cur_tick: 0,
            points,
            track_name,
            max_steering_angle,
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

    fn telemetry(&mut self) -> Result<TelemetryData, OcypodeError> {
        if self.cur_tick >= self.points.len() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "End of points vec".to_string(),
            });
        }

        let point = self.points[self.cur_tick].clone();
        self.cur_tick += 1;

        Ok(point)
    }

    fn game_source(&self) -> GameSource {
        self.game_source
    }
}
