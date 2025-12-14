// Track metadata management module
// Provides functionality for creating, storing, and retrieving track metadata
// including SVG maps and corner annotations

pub mod storage;
pub mod svg_generator;
pub mod types;

// Re-export commonly used types
pub use storage::{FileBasedStorage, TrackMetadataStorage};
pub use svg_generator::{
    ScalingAlgorithm, TrackMapConfig, TrackMapGenerator, TrackPositionMapping,
};
pub use types::{CornerAnnotation, CornerType, TrackMetadata};
