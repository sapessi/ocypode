// Integration test for track metadata functionality

use ocypode::track_metadata::FileBasedStorage;
use ocypode::{CornerAnnotation, CornerType, TrackMetadata, TrackMetadataStorage};
use tempfile::TempDir;

#[test]
fn test_track_metadata_integration() {
    // Create temporary storage
    let temp_dir = TempDir::new().unwrap();
    let mut storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Create track metadata
    let mut metadata = TrackMetadata::new(
        "Silverstone GP".to_string(),
        "silverstone_gp".to_string(),
        "<svg><path d=\"M 100,100 L 200,200\" /></svg>".to_string(),
    );

    // Add corner annotations
    let corner1 = CornerAnnotation::new(1, 0.1, 0.15, CornerType::RightHand).unwrap();
    let corner2 = CornerAnnotation::new(2, 0.3, 0.35, CornerType::LeftHand).unwrap();

    metadata.add_corner(corner1);
    metadata.add_corner(corner2);

    // Save metadata
    storage.save_metadata(&metadata).unwrap();

    // Verify it can be loaded
    let loaded = storage.load_metadata("silverstone_gp").unwrap();
    assert!(loaded.is_some());

    let loaded_metadata = loaded.unwrap();
    assert_eq!(loaded_metadata.track_name, "Silverstone GP");
    assert_eq!(loaded_metadata.corners.len(), 2);

    // Verify corner validation works
    assert!(loaded_metadata.validate_corners().is_ok());

    // List tracks
    let tracks = storage.list_available_tracks().unwrap();
    assert_eq!(tracks.len(), 1);
    assert!(tracks.contains(&"Silverstone GP".to_string()));
}
