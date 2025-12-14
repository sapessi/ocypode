// Library interface for ocypode
// This allows integration tests to access internal modules

pub mod errors;
pub mod setup_assistant;
pub mod telemetry;
pub mod track_metadata;

// Re-export commonly used types
pub use errors::OcypodeError;
pub use setup_assistant::{CornerPhase, FindingType, SetupAssistant};
pub use telemetry::{SessionInfo, TelemetryData, TelemetryOutput};
pub use track_metadata::{CornerAnnotation, CornerType, TrackMetadata, TrackMetadataStorage};
