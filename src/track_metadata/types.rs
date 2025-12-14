// Core data structures for track metadata management

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Complete metadata for a racing track including SVG map and corner annotations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TrackMetadata {
    /// Human-readable track name (e.g., "Silverstone GP")
    pub track_name: String,
    /// Unique track identifier for storage and lookup
    pub track_id: String,
    /// SVG representation of the track layout
    pub svg_map: String,
    /// List of corner annotations with positions and metadata
    pub corners: Vec<CornerAnnotation>,
    /// Timestamp when metadata was first created
    pub created_at: SystemTime,
    /// Timestamp when metadata was last updated
    pub updated_at: SystemTime,
    /// Version number for metadata format compatibility
    pub version: u32,
}

impl TrackMetadata {
    /// Create new track metadata with current timestamp
    pub fn new(track_name: String, track_id: String, svg_map: String) -> Self {
        let now = SystemTime::now();
        Self {
            track_name,
            track_id,
            svg_map,
            corners: Vec::new(),
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Update the metadata timestamp and increment version
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now();
        self.version += 1;
    }

    /// Add a corner annotation to the track
    pub fn add_corner(&mut self, corner: CornerAnnotation) {
        self.corners.push(corner);
        self.touch();
    }

    /// Remove a corner annotation by corner number
    pub fn remove_corner(&mut self, corner_number: u32) -> bool {
        let initial_len = self.corners.len();
        self.corners.retain(|c| c.corner_number != corner_number);
        if self.corners.len() != initial_len {
            self.touch();
            true
        } else {
            false
        }
    }

    /// Get corner annotation by corner number
    pub fn get_corner(&self, corner_number: u32) -> Option<&CornerAnnotation> {
        self.corners
            .iter()
            .find(|c| c.corner_number == corner_number)
    }

    /// Validate that corner numbers are unique and ranges don't overlap
    pub fn validate_corners(&self) -> Result<(), String> {
        // Check for duplicate corner numbers
        let mut corner_numbers = std::collections::HashSet::new();
        for corner in &self.corners {
            if !corner_numbers.insert(corner.corner_number) {
                return Err(format!("Duplicate corner number: {}", corner.corner_number));
            }
        }

        // Check for overlapping ranges
        for (i, corner1) in self.corners.iter().enumerate() {
            for corner2 in self.corners.iter().skip(i + 1) {
                if corner1.overlaps_with(corner2) {
                    return Err(format!(
                        "Corner {} overlaps with corner {}",
                        corner1.corner_number, corner2.corner_number
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Annotation for a specific corner on the track
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CornerAnnotation {
    /// Unique corner number within the track
    pub corner_number: u32,
    /// Track percentage where corner begins (0.0-1.0)
    pub track_percentage_start: f32,
    /// Track percentage where corner ends (0.0-1.0)
    pub track_percentage_end: f32,
    /// Type/classification of the corner
    pub corner_type: CornerType,
    /// Optional description or notes about the corner
    pub description: Option<String>,
}

impl CornerAnnotation {
    /// Create a new corner annotation
    pub fn new(
        corner_number: u32,
        track_percentage_start: f32,
        track_percentage_end: f32,
        corner_type: CornerType,
    ) -> Result<Self, String> {
        // Validate track percentages
        if track_percentage_start < 0.0 || track_percentage_start > 1.0 {
            return Err("track_percentage_start must be between 0.0 and 1.0".to_string());
        }
        if track_percentage_end < 0.0 || track_percentage_end > 1.0 {
            return Err("track_percentage_end must be between 0.0 and 1.0".to_string());
        }
        if track_percentage_start >= track_percentage_end {
            return Err(
                "track_percentage_start must be less than track_percentage_end".to_string(),
            );
        }

        Ok(Self {
            corner_number,
            track_percentage_start,
            track_percentage_end,
            corner_type,
            description: None,
        })
    }

    /// Set optional description for the corner
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Check if this corner's range overlaps with another corner
    pub fn overlaps_with(&self, other: &CornerAnnotation) -> bool {
        // Two ranges overlap if one starts before the other ends
        self.track_percentage_start < other.track_percentage_end
            && other.track_percentage_start < self.track_percentage_end
    }

    /// Check if a track percentage falls within this corner's range
    pub fn contains_percentage(&self, track_percentage: f32) -> bool {
        track_percentage >= self.track_percentage_start
            && track_percentage <= self.track_percentage_end
    }

    /// Get the length of this corner as a percentage of the track
    pub fn length(&self) -> f32 {
        self.track_percentage_end - self.track_percentage_start
    }
}

/// Classification of corner types for analysis purposes
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CornerType {
    /// Left-hand turn
    LeftHand,
    /// Right-hand turn
    RightHand,
    /// Series of alternating turns
    Chicane,
    /// Very tight turn (typically > 90 degrees)
    Hairpin,
}

impl CornerType {
    /// Get a human-readable description of the corner type
    pub fn description(&self) -> &'static str {
        match self {
            CornerType::LeftHand => "Left-hand turn",
            CornerType::RightHand => "Right-hand turn",
            CornerType::Chicane => "Chicane",
            CornerType::Hairpin => "Hairpin turn",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_metadata_creation() {
        let metadata = TrackMetadata::new(
            "Test Track".to_string(),
            "test_track".to_string(),
            "<svg></svg>".to_string(),
        );

        assert_eq!(metadata.track_name, "Test Track");
        assert_eq!(metadata.track_id, "test_track");
        assert_eq!(metadata.svg_map, "<svg></svg>");
        assert_eq!(metadata.corners.len(), 0);
        assert_eq!(metadata.version, 1);
    }

    #[test]
    fn test_corner_annotation_creation() {
        let corner = CornerAnnotation::new(1, 0.1, 0.2, CornerType::LeftHand).unwrap();

        assert_eq!(corner.corner_number, 1);
        assert_eq!(corner.track_percentage_start, 0.1);
        assert_eq!(corner.track_percentage_end, 0.2);
        assert_eq!(corner.corner_type, CornerType::LeftHand);
        assert!(corner.description.is_none());
    }

    #[test]
    fn test_corner_annotation_validation() {
        // Invalid start percentage
        assert!(CornerAnnotation::new(1, -0.1, 0.2, CornerType::LeftHand).is_err());

        // Invalid end percentage
        assert!(CornerAnnotation::new(1, 0.1, 1.1, CornerType::LeftHand).is_err());

        // Start >= end
        assert!(CornerAnnotation::new(1, 0.2, 0.1, CornerType::LeftHand).is_err());
        assert!(CornerAnnotation::new(1, 0.2, 0.2, CornerType::LeftHand).is_err());
    }

    #[test]
    fn test_corner_overlap_detection() {
        let corner1 = CornerAnnotation::new(1, 0.1, 0.3, CornerType::LeftHand).unwrap();
        let corner2 = CornerAnnotation::new(2, 0.2, 0.4, CornerType::RightHand).unwrap();
        let corner3 = CornerAnnotation::new(3, 0.5, 0.7, CornerType::Hairpin).unwrap();

        // corner1 and corner2 overlap
        assert!(corner1.overlaps_with(&corner2));
        assert!(corner2.overlaps_with(&corner1));

        // corner1 and corner3 don't overlap
        assert!(!corner1.overlaps_with(&corner3));
        assert!(!corner3.overlaps_with(&corner1));
    }

    #[test]
    fn test_track_metadata_corner_management() {
        let mut metadata = TrackMetadata::new(
            "Test Track".to_string(),
            "test_track".to_string(),
            "<svg></svg>".to_string(),
        );

        let corner = CornerAnnotation::new(1, 0.1, 0.2, CornerType::LeftHand).unwrap();
        metadata.add_corner(corner);

        assert_eq!(metadata.corners.len(), 1);
        assert!(metadata.get_corner(1).is_some());
        assert!(metadata.get_corner(2).is_none());

        assert!(metadata.remove_corner(1));
        assert_eq!(metadata.corners.len(), 0);
        assert!(!metadata.remove_corner(1)); // Already removed
    }

    #[test]
    fn test_track_metadata_validation() {
        let mut metadata = TrackMetadata::new(
            "Test Track".to_string(),
            "test_track".to_string(),
            "<svg></svg>".to_string(),
        );

        // Add valid corners
        let corner1 = CornerAnnotation::new(1, 0.1, 0.2, CornerType::LeftHand).unwrap();
        let corner2 = CornerAnnotation::new(2, 0.3, 0.4, CornerType::RightHand).unwrap();
        metadata.add_corner(corner1);
        metadata.add_corner(corner2);

        assert!(metadata.validate_corners().is_ok());

        // Add duplicate corner number
        let corner3 = CornerAnnotation::new(1, 0.5, 0.6, CornerType::Hairpin).unwrap();
        metadata.add_corner(corner3);

        assert!(metadata.validate_corners().is_err());
    }
}
