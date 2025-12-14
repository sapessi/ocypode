use std::time::Duration;

use log::error;

use crate::OcypodeError;

use super::{GameSource, SessionInfo, TelemetryData, TelemetryOutput};

#[allow(unused)]
const CONN_RETRY_WAIT_MS: u64 = 200;
#[allow(unused)]
const MAX_STEERING_ANGLE_DEFAULT: f32 = std::f32::consts::PI;
pub(crate) const CONN_RETRY_MAX_WAIT_S: u64 = 600;

/// Extract track name from ACC static information with comprehensive error handling and fallback
#[cfg(windows)]
fn extract_acc_track_name(
    state: &simetry::assetto_corsa_competizione::SimState,
) -> Result<String, OcypodeError> {
    use log::{debug, error, info, warn};

    // ACC static data contains track information
    // Access the track name from the static data structure
    let raw_track_name = &state.static_data.track;

    // Log the raw track name for debugging
    debug!("ACC: Raw track name from static data: '{}'", raw_track_name);

    // Check if we have a valid track name
    if raw_track_name.is_empty() {
        warn!("ACC: Empty track name in static data, attempting fallback identification");

        // Fallback 1: Try to extract from track configuration
        let track_config = &state.static_data.track_configuration;
        if !track_config.is_empty() {
            debug!(
                "ACC: Attempting to derive track name from configuration: '{}'",
                track_config
            );
            let derived_name = derive_track_name_from_config(track_config);
            if !derived_name.is_empty() {
                debug!(
                    "ACC: Successfully derived track name from configuration: '{}'",
                    derived_name
                );
                return normalize_and_validate_track_name(&derived_name);
            }
        }

        // Fallback 2: Manual track identification required
        error!("ACC: Unable to extract track name from static data or configuration");
        return Err(OcypodeError::ManualTrackIdentificationRequired {
            reason: "Track name is empty in ACC static data and could not be derived from configuration. Please manually identify the track or restart the session.".to_string()
        });
    }

    // Normalize and validate the track name with comprehensive error handling
    normalize_and_validate_track_name(raw_track_name)
}

/// Derive track name from track configuration string
#[cfg(windows)]
fn derive_track_name_from_config(config: &str) -> String {
    use log::debug;

    debug!(
        "ACC: Attempting to derive track name from config: '{}'",
        config
    );

    // Common configuration patterns to track name mappings
    let config_mappings = [
        ("spa", "Spa-Francorchamps"),
        ("silverstone", "Silverstone GP"),
        ("monza", "Monza"),
        ("nurburgring", "Nürburgring GP"),
        ("brands_hatch", "Brands Hatch GP"),
        ("paul_ricard", "Paul Ricard"),
        ("misano", "Misano World Circuit"),
        ("zolder", "Zolder"),
        ("barcelona", "Barcelona"),
        ("hungaroring", "Hungaroring"),
        ("zandvoort", "Zandvoort"),
        ("imola", "Imola"),
        ("kyalami", "Kyalami"),
        ("laguna_seca", "Laguna Seca"),
        ("mount_panorama", "Mount Panorama"),
        ("suzuka", "Suzuka"),
        ("donington", "Donington Park"),
        ("snetterton", "Snetterton"),
        ("oulton_park", "Oulton Park"),
        ("watkins_glen", "Watkins Glen"),
        ("cota", "Circuit of the Americas"),
        ("indianapolis", "Indianapolis Motor Speedway"),
    ];

    let config_lower = config.to_lowercase();

    for (pattern, track_name) in &config_mappings {
        if config_lower.contains(pattern) {
            debug!(
                "ACC: Matched config pattern '{}' to track '{}'",
                pattern, track_name
            );
            return track_name.to_string();
        }
    }

    debug!("ACC: No matching pattern found for config '{}'", config);
    String::new()
}

/// Normalize and validate track name with comprehensive error handling
#[cfg(windows)]
fn normalize_and_validate_track_name(track_name: &str) -> Result<String, OcypodeError> {
    use log::{debug, warn};

    debug!("ACC: Normalizing track name: '{}'", track_name);

    // Normalize the track name
    let normalized_name = match normalize_acc_track_name(track_name) {
        Ok(name) => name,
        Err(e) => {
            warn!(
                "ACC: Track name normalization failed for '{}': {}",
                track_name, e
            );
            return Err(OcypodeError::TrackNameExtractionError {
                reason: format!("Failed to normalize track name '{}': {}", track_name, e),
            });
        }
    };

    debug!("ACC: Normalized track name: '{}'", normalized_name);

    // Validate the normalized name
    if let Err(e) = validate_acc_track_name(&normalized_name) {
        warn!(
            "ACC: Track name validation failed for '{}': {}",
            normalized_name, e
        );
        // Don't fail on validation - just log the warning and continue
        // This allows for custom/unknown tracks to work
    }

    Ok(normalized_name)
}

/// Normalize ACC track names to consistent identifiers with comprehensive error handling
#[cfg(windows)]
fn normalize_acc_track_name(track_name: &str) -> Result<String, OcypodeError> {
    use log::{debug, warn};

    // Input validation
    if track_name.is_empty() {
        return Err(OcypodeError::InvalidUserInput {
            field: "track_name".to_string(),
            reason: "Track name cannot be empty".to_string(),
        });
    }

    if track_name.len() > 200 {
        warn!(
            "ACC: Unusually long track name ({}): '{}'",
            track_name.len(),
            track_name
        );
        return Err(OcypodeError::InvalidUserInput {
            field: "track_name".to_string(),
            reason: format!(
                "Track name too long ({} characters, max 200)",
                track_name.len()
            ),
        });
    }

    debug!("ACC: Normalizing track name: '{}'", track_name);

    // Remove common prefixes/suffixes and normalize spacing
    let normalized = track_name
        .trim()
        .to_lowercase()
        .replace("_", " ")
        .replace("-", " ")
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>();

    if normalized.is_empty() {
        return Err(OcypodeError::InvalidUserInput {
            field: "track_name".to_string(),
            reason: "Track name contains no valid characters".to_string(),
        });
    }

    debug!("ACC: Normalized input: '{}'", normalized);

    // Map known ACC track variations to standard names
    let standard_name = match normalized.as_str() {
        "spa francorchamps" | "spa" | "circuit de spa francorchamps" => "Spa-Francorchamps",
        "silverstone" | "silverstone circuit" | "silverstone gp" => "Silverstone GP",
        "monza" | "autodromo nazionale monza" | "monza circuit" => "Monza",
        "nurburgring" | "nurburgring gp" | "nurburgring grand prix" => "Nürburgring GP",
        "brands hatch" | "brands hatch gp" => "Brands Hatch GP",
        "paul ricard" | "circuit paul ricard" => "Paul Ricard",
        "misano" | "misano world circuit" => "Misano World Circuit",
        "zolder" | "circuit zolder" => "Zolder",
        "barcelona" | "circuit de barcelona catalunya" => "Barcelona",
        "hungaroring" | "hungaroring circuit" => "Hungaroring",
        "zandvoort" | "circuit zandvoort" => "Zandvoort",
        "imola" | "autodromo enzo e dino ferrari" => "Imola",
        "kyalami" | "kyalami grand prix circuit" => "Kyalami",
        "laguna seca" | "weathertech raceway laguna seca" => "Laguna Seca",
        "mount panorama" | "bathurst" | "mount panorama circuit" => "Mount Panorama",
        "suzuka" | "suzuka circuit" => "Suzuka",
        "donington" | "donington park" => "Donington Park",
        "snetterton" | "snetterton circuit" => "Snetterton",
        "oulton park" | "oulton park circuit" => "Oulton Park",
        "watkins glen" | "watkins glen international" => "Watkins Glen",
        "cota" | "circuit of the americas" => "Circuit of the Americas",
        "indianapolis" | "indianapolis motor speedway" => "Indianapolis Motor Speedway",
        _ => {
            // If no specific mapping found, create a proper title case version
            debug!(
                "ACC: No specific mapping found, creating title case for: '{}'",
                normalized
            );
            return Ok(create_title_case(&normalized));
        }
    };

    debug!("ACC: Final normalized name: '{}'", standard_name);
    Ok(standard_name.to_string())
}

/// Create title case from normalized track name
#[cfg(windows)]
fn create_title_case(input: &str) -> String {
    // Simple title case implementation
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Validate ACC track name against known identifiers
#[cfg(windows)]
fn validate_acc_track_name(track_name: &str) -> Result<(), OcypodeError> {
    // List of known ACC tracks for validation
    let known_tracks = [
        "Spa-Francorchamps",
        "Silverstone GP",
        "Monza",
        "Nürburgring GP",
        "Brands Hatch GP",
        "Paul Ricard",
        "Misano World Circuit",
        "Zolder",
        "Barcelona",
        "Hungaroring",
        "Zandvoort",
        "Imola",
        "Kyalami",
        "Laguna Seca",
        "Mount Panorama",
        "Suzuka",
        "Donington Park",
        "Snetterton",
        "Oulton Park",
        "Watkins Glen",
        "Circuit of the Americas",
        "Indianapolis Motor Speedway",
        "Unknown",
    ];

    if !known_tracks.contains(&track_name) {
        use log::warn;
        warn!("ACC: Unknown track name '{}', using as-is", track_name);
    }

    Ok(())
}

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
    #[allow(dead_code)]
    fn game_source(&self) -> GameSource;
}

#[cfg(windows)]
#[allow(unused)]
pub(crate) struct IRacingTelemetryProducer {
    client: Option<simetry::iracing::Client>,
    retry_wait_ms: u64,
    _retry_timeout_s: u64,
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
    #[allow(unused)]
    pub fn new(retry_wait_ms: u64, retry_timeout_s: u64) -> Self {
        Self {
            client: None,
            retry_wait_ms,
            _retry_timeout_s: retry_timeout_s,
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
#[allow(unused)]
pub(crate) struct ACCTelemetryProducer {
    client: Option<simetry::assetto_corsa_competizione::Client>,
    retry_wait_ms: u64,
    _retry_timeout_s: u64,
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
    #[allow(unused)]
    pub fn new(retry_wait_ms: u64, retry_timeout_s: u64) -> Self {
        Self {
            client: None,
            retry_wait_ms,
            _retry_timeout_s: retry_timeout_s,
            point_no: 0,
        }
    }
}

#[cfg(windows)]
impl TelemetryProducer for ACCTelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError> {
        use log::info;

        info!("ACC: Starting connection to shared memory...");
        let retry_delay = Duration::from_millis(self.retry_wait_ms);

        let client = tokio::runtime::Runtime::new().unwrap().block_on(
            simetry::assetto_corsa_competizione::Client::connect(retry_delay),
        );

        self.client = Some(client);
        info!("ACC: Connection established successfully");
        Ok(())
    }

    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError> {
        use log::error;

        if self.client.is_none() {
            error!("ACC: Client not initialized when requesting session info");
            return Err(OcypodeError::TelemetryProducerError {
                description: "The ACC connection is not initialized, call start() first."
                    .to_string(),
            });
        }

        let client = self.client.as_mut().expect("Missing ACC connection");

        // In simetry 0.2.3, use next_sim_state() to get the current state
        let state = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.next_sim_state())
            .ok_or_else(|| {
                error!("ACC: Could not retrieve state - game may not be in an active session");
                OcypodeError::TelemetryProducerError {
                    description: "Could not retrieve ACC state".to_string(),
                }
            })?;

        // Extract track name from ACC static data
        let track_name = extract_acc_track_name(&state)?;

        // Extract track configuration if available
        let track_config = state.static_data.track_configuration.clone();

        // Track length is not directly available in ACC static data
        // We could potentially calculate it from telemetry data later
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
        use log::{debug, error};

        if self.client.is_none() {
            error!("ACC: Client not initialized when requesting telemetry");
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
            .ok_or_else(|| {
                error!(
                    "ACC: Could not retrieve telemetry data - game may have closed or session ended"
                );
                OcypodeError::TelemetryProducerError {
                    description: "Could not retrieve ACC telemetry".to_string(),
                }
            })?;

        if self.point_no.is_multiple_of(100) {
            debug!(
                "ACC: Successfully retrieved telemetry point #{}",
                self.point_no
            );
        }

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
                    points.push(*telemetry);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_normalize_acc_track_name() {
        // Test known track mappings
        assert_eq!(
            normalize_acc_track_name("spa francorchamps").unwrap(),
            "Spa-Francorchamps"
        );
        assert_eq!(
            normalize_acc_track_name("SPA").unwrap(),
            "Spa-Francorchamps"
        );
        assert_eq!(
            normalize_acc_track_name("silverstone gp").unwrap(),
            "Silverstone GP"
        );
        assert_eq!(normalize_acc_track_name("monza").unwrap(), "Monza");
        assert_eq!(
            normalize_acc_track_name("nurburgring gp").unwrap(),
            "Nürburgring GP"
        );

        // Test with underscores and hyphens
        assert_eq!(
            normalize_acc_track_name("spa_francorchamps").unwrap(),
            "Spa-Francorchamps"
        );
        assert_eq!(
            normalize_acc_track_name("brands-hatch").unwrap(),
            "Brands Hatch GP"
        );

        // Test unknown track (should return original)
        assert_eq!(
            normalize_acc_track_name("Custom Track").unwrap(),
            "Custom Track"
        );

        // Test empty string (should now return error)
        assert!(normalize_acc_track_name("").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_validate_acc_track_name() {
        // Known tracks should validate successfully
        assert!(validate_acc_track_name("Spa-Francorchamps").is_ok());
        assert!(validate_acc_track_name("Silverstone GP").is_ok());
        assert!(validate_acc_track_name("Unknown").is_ok());

        // Unknown tracks should still validate (with warning)
        assert!(validate_acc_track_name("Custom Track").is_ok());

        // Empty string should validate
        assert!(validate_acc_track_name("").is_ok());
    }

    #[test]
    fn test_mock_producer_with_acc_track_name() {
        // Test that MockTelemetryProducer can simulate ACC with proper track names
        let mock_data = vec![TelemetryData {
            point_no: 1,
            timestamp_ms: 1000,
            game_source: GameSource::ACC,
            gear: Some(3),
            speed_mps: Some(50.0),
            engine_rpm: Some(6000.0),
            max_engine_rpm: Some(7300.0),
            shift_point_rpm: None,
            throttle: Some(0.8),
            brake: Some(0.0),
            clutch: Some(0.0),
            steering_angle_rad: Some(0.1),
            steering_pct: Some(0.1),
            lap_distance_m: None,
            lap_distance_pct: Some(0.5),
            lap_number: Some(1),
            last_lap_time_s: Some(120.0),
            best_lap_time_s: Some(118.5),
            is_pit_limiter_engaged: Some(false),
            is_in_pit_lane: Some(false),
            is_abs_active: Some(false),
            latitude_deg: None,
            longitude_deg: None,
            lateral_accel_mps2: None,
            longitudinal_accel_mps2: None,
            pitch_rad: Some(0.0),
            pitch_rate_rps: None,
            roll_rad: Some(0.0),
            roll_rate_rps: None,
            yaw_rad: Some(0.0),
            yaw_rate_rps: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
            world_position_x: Some(100.0),
            world_position_y: Some(200.0),
            world_position_z: Some(0.0),
            world_velocity_x: Some(10.0),
            world_velocity_y: Some(20.0),
            world_velocity_z: Some(0.0),
            track_position_pct: Some(0.5),
            track_sector: None,
        }];

        let mut producer = MockTelemetryProducer::from_points(mock_data);
        producer.track_name = "Spa-Francorchamps".to_string(); // Simulate extracted track name
        producer.game_source = GameSource::ACC;

        // Test that the producer works correctly
        assert!(producer.start().is_ok());

        let session_info = producer.session_info().unwrap();
        assert_eq!(session_info.track_name, "Spa-Francorchamps");
        assert_eq!(session_info.game_source, GameSource::ACC);

        // Test that telemetry data is returned correctly
        let telemetry = producer.telemetry().unwrap();
        assert_eq!(telemetry.game_source, GameSource::ACC);
        assert_eq!(telemetry.point_no, 1);
    }
}
