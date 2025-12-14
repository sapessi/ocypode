// Storage implementation for track metadata persistence

use crate::errors::OcypodeError;
use crate::track_metadata::types::TrackMetadata;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Trait defining the interface for track metadata storage operations
pub trait TrackMetadataStorage {
    /// Save track metadata to persistent storage
    fn save_metadata(&mut self, metadata: &TrackMetadata) -> Result<(), OcypodeError>;

    /// Load track metadata by track name/ID
    fn load_metadata(&self, track_name: &str) -> Result<Option<TrackMetadata>, OcypodeError>;

    /// List all available track names in storage
    fn list_available_tracks(&self) -> Result<Vec<String>, OcypodeError>;

    /// Delete track metadata from storage
    fn delete_metadata(&mut self, track_name: &str) -> Result<(), OcypodeError>;

    /// Check if metadata exists for a given track
    fn metadata_exists(&self, track_name: &str) -> Result<bool, OcypodeError>;
}

/// File-based implementation of track metadata storage
pub struct FileBasedStorage {
    /// Base directory for storing track metadata files
    storage_path: PathBuf,
    /// In-memory cache of loaded metadata for performance
    cache: HashMap<String, TrackMetadata>,
}

impl FileBasedStorage {
    /// Create a new file-based storage instance
    pub fn new(storage_path: PathBuf) -> Result<Self, OcypodeError> {
        // Ensure the storage directory exists
        if !storage_path.exists() {
            fs::create_dir_all(&storage_path)
                .map_err(|e| OcypodeError::ConfigIOError { source: e })?;
        }

        Ok(Self {
            storage_path,
            cache: HashMap::new(),
        })
    }

    /// Create storage in the default application data directory
    pub fn new_default() -> Result<Self, OcypodeError> {
        let storage_path = Self::default_storage_path()?;
        Self::new(storage_path)
    }

    /// Get the default storage path for track metadata
    pub fn default_storage_path() -> Result<PathBuf, OcypodeError> {
        let app_data_dir = dirs::data_dir().ok_or(OcypodeError::NoConfigDir)?;
        Ok(app_data_dir.join("ocypode").join("track_metadata"))
    }

    /// Generate file path for a given track name
    fn file_path_for_track(&self, track_name: &str) -> PathBuf {
        let filename = format!("{}.json", Self::normalize_track_name(track_name));
        self.storage_path.join(filename)
    }

    /// Normalize track name for consistent file naming
    fn normalize_track_name(track_name: &str) -> String {
        track_name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }

    /// Load metadata from file without caching
    fn load_from_file(&self, track_name: &str) -> Result<Option<TrackMetadata>, OcypodeError> {
        let file_path = self.file_path_for_track(track_name);

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?;

        let metadata: TrackMetadata = serde_json::from_str(&content)
            .map_err(|e| OcypodeError::ConfigSerializeError { source: e })?;

        Ok(Some(metadata))
    }

    /// Load metadata from file with recovery mechanisms
    fn load_from_file_with_recovery(
        &self,
        track_name: &str,
    ) -> Result<Option<TrackMetadata>, OcypodeError> {
        use log::{debug, warn};

        let file_path = self.file_path_for_track(track_name);

        if !file_path.exists() {
            debug!("Metadata file does not exist: {:?}", file_path);
            return Ok(None);
        }

        // First attempt: load normally
        match self.attempt_load_from_file(&file_path) {
            Ok(metadata) => {
                debug!("Successfully loaded metadata on first attempt");
                return Ok(Some(metadata));
            }
            Err(e) => {
                warn!("First load attempt failed: {}", e);

                // Second attempt: try to load from backup
                if let Ok(metadata) = self.attempt_load_from_backup(track_name) {
                    warn!("Successfully loaded from backup after primary file failed");
                    return Ok(Some(metadata));
                }

                // Third attempt: try to repair the file
                if let Ok(metadata) = self.attempt_repair_and_load(&file_path) {
                    warn!("Successfully loaded after attempting file repair");
                    return Ok(Some(metadata));
                }

                // All attempts failed
                return Err(OcypodeError::TrackMetadataStorageError {
                    reason: format!("Failed to load metadata after all recovery attempts: {}", e),
                });
            }
        }
    }

    /// Attempt to load metadata from a specific file
    fn attempt_load_from_file(
        &self,
        file_path: &std::path::Path,
    ) -> Result<TrackMetadata, OcypodeError> {
        let content =
            fs::read_to_string(file_path).map_err(|e| OcypodeError::FileOperationError {
                operation: "read_metadata_file".to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;

        if content.is_empty() {
            return Err(OcypodeError::TrackMetadataStorageError {
                reason: "Metadata file is empty".to_string(),
            });
        }

        let metadata: TrackMetadata = serde_json::from_str(&content).map_err(|e| {
            OcypodeError::TrackMetadataStorageError {
                reason: format!("Failed to parse JSON: {}", e),
            }
        })?;

        Ok(metadata)
    }

    /// Attempt to load from backup files
    fn attempt_load_from_backup(&self, track_name: &str) -> Result<TrackMetadata, OcypodeError> {
        use std::fs;

        let file_path = self.file_path_for_track(track_name);
        let parent_dir = file_path
            .parent()
            .ok_or_else(|| OcypodeError::FileOperationError {
                operation: "load_backup".to_string(),
                reason: "Cannot determine parent directory".to_string(),
            })?;

        let entries = fs::read_dir(parent_dir).map_err(|e| OcypodeError::FileOperationError {
            operation: "load_backup".to_string(),
            reason: format!("Cannot read directory: {}", e),
        })?;

        let mut backup_files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(&format!(
                        "{}.backup.",
                        Self::normalize_track_name(track_name)
                    )) {
                        backup_files.push(path);
                    }
                }
            }
        }

        // Try backups from newest to oldest
        backup_files.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        backup_files.reverse();

        for backup_path in backup_files {
            if let Ok(metadata) = self.attempt_load_from_file(&backup_path) {
                log::info!("Successfully loaded from backup: {:?}", backup_path);
                return Ok(metadata);
            }
        }

        Err(OcypodeError::TrackMetadataStorageError {
            reason: "No valid backup files found".to_string(),
        })
    }

    /// Attempt to repair a corrupted file and load it
    fn attempt_repair_and_load(
        &self,
        file_path: &std::path::Path,
    ) -> Result<TrackMetadata, OcypodeError> {
        use std::fs;

        let content =
            fs::read_to_string(file_path).map_err(|e| OcypodeError::FileOperationError {
                operation: "repair_file".to_string(),
                reason: format!("Failed to read file for repair: {}", e),
            })?;

        // Try to repair common JSON issues
        let repaired_content = self.attempt_json_repair(&content);

        if repaired_content != content {
            log::info!("Attempting to parse repaired JSON content");

            if let Ok(metadata) = serde_json::from_str::<TrackMetadata>(&repaired_content) {
                log::info!("Successfully parsed repaired JSON");

                // Save the repaired version
                let repair_path = file_path.with_extension("json.repaired");
                if let Err(e) = fs::write(&repair_path, &repaired_content) {
                    log::warn!("Failed to save repaired file: {}", e);
                }

                return Ok(metadata);
            }
        }

        Err(OcypodeError::TrackMetadataStorageError {
            reason: "File repair attempt failed".to_string(),
        })
    }

    /// Attempt basic JSON repair
    fn attempt_json_repair(&self, content: &str) -> String {
        let mut repaired = content.to_string();

        // Remove null bytes
        repaired = repaired.replace('\0', "");

        // Fix common trailing comma issues
        repaired = repaired.replace(",}", "}");
        repaired = repaired.replace(",]", "]");

        // Ensure proper line endings
        repaired = repaired.replace('\r', "");

        repaired
    }

    /// Validate loaded metadata for consistency
    fn validate_loaded_metadata(&self, metadata: &TrackMetadata) -> Result<(), OcypodeError> {
        use log::debug;

        debug!("Validating loaded metadata for: {}", metadata.track_name);

        // Basic field validation
        if metadata.track_name.is_empty() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "Loaded metadata has empty track name".to_string(),
            });
        }

        if metadata.track_id.is_empty() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "Loaded metadata has empty track ID".to_string(),
            });
        }

        if metadata.svg_map.is_empty() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "Loaded metadata has empty SVG map".to_string(),
            });
        }

        // Validate corner data
        if let Err(e) = metadata.validate_corners() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: format!("Loaded metadata has invalid corners: {}", e),
            });
        }

        // Check for reasonable data sizes
        if metadata.svg_map.len() > 1_000_000 {
            // 1MB limit
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: format!(
                    "SVG map too large ({} bytes, max 1MB)",
                    metadata.svg_map.len()
                ),
            });
        }

        if metadata.corners.len() > 100 {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: format!("Too many corners ({}, max 100)", metadata.corners.len()),
            });
        }

        debug!("Loaded metadata validation passed");
        Ok(())
    }

    /// Save metadata to file
    fn save_to_file(&self, metadata: &TrackMetadata) -> Result<(), OcypodeError> {
        let file_path = self.file_path_for_track(&metadata.track_id);

        let content = serde_json::to_string_pretty(metadata)
            .map_err(|e| OcypodeError::ConfigSerializeError { source: e })?;

        fs::write(&file_path, content).map_err(|e| OcypodeError::ConfigIOError { source: e })?;

        Ok(())
    }

    /// Clear the in-memory cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the storage directory path
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    /// Comprehensive validation of metadata before saving
    fn validate_metadata_for_save(&self, metadata: &TrackMetadata) -> Result<(), OcypodeError> {
        use log::debug;

        debug!("Validating metadata for track: {}", metadata.track_name);

        // Validate basic fields
        if metadata.track_name.is_empty() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "Track name cannot be empty".to_string(),
            });
        }

        if metadata.track_id.is_empty() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "Track ID cannot be empty".to_string(),
            });
        }

        if metadata.svg_map.is_empty() {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "SVG map cannot be empty".to_string(),
            });
        }

        // Validate SVG format
        if !metadata.svg_map.trim_start().starts_with("<svg") {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "SVG map must be valid SVG format starting with <svg".to_string(),
            });
        }

        if !metadata.svg_map.trim_end().ends_with("</svg>") {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: "SVG map must be valid SVG format ending with </svg>".to_string(),
            });
        }

        // Validate track name format
        if metadata.track_name.len() > 100 {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: format!(
                    "Track name too long ({} characters, max 100)",
                    metadata.track_name.len()
                ),
            });
        }

        // Validate track ID format (should be filesystem-safe)
        if !metadata
            .track_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason:
                    "Track ID must contain only alphanumeric characters, underscores, and hyphens"
                        .to_string(),
            });
        }

        // Validate corners
        metadata
            .validate_corners()
            .map_err(|e| OcypodeError::TrackMetadataValidationError { reason: e })?;

        // Validate corner count
        if metadata.corners.len() > 50 {
            return Err(OcypodeError::TrackMetadataValidationError {
                reason: format!("Too many corners ({}, max 50)", metadata.corners.len()),
            });
        }

        debug!(
            "Metadata validation passed for track: {}",
            metadata.track_name
        );
        Ok(())
    }

    /// Check available disk space before saving
    fn check_available_disk_space(&self) -> Result<(), OcypodeError> {
        use std::fs;

        // Try to get filesystem stats (this is a simplified check)
        match fs::metadata(&self.storage_path) {
            Ok(_) => {
                // In a full implementation, you would check actual available space
                // For now, we just verify the directory is accessible
                Ok(())
            }
            Err(e) => Err(OcypodeError::FileOperationError {
                operation: "check_disk_space".to_string(),
                reason: format!("Cannot access storage directory: {}", e),
            }),
        }
    }

    /// Create backup of existing metadata if it exists
    fn create_backup_if_exists(&self, track_id: &str) -> Result<(), OcypodeError> {
        use std::fs;
        use std::time::SystemTime;

        let file_path = self.file_path_for_track(track_id);

        if file_path.exists() {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| OcypodeError::FileOperationError {
                    operation: "create_backup".to_string(),
                    reason: format!("Failed to get timestamp: {}", e),
                })?
                .as_secs();

            let backup_path = file_path.with_extension(format!("json.backup.{}", timestamp));

            fs::copy(&file_path, &backup_path).map_err(|e| OcypodeError::FileOperationError {
                operation: "create_backup".to_string(),
                reason: format!("Failed to create backup: {}", e),
            })?;

            log::info!("Created backup: {:?}", backup_path);
        }

        Ok(())
    }

    /// Save metadata to file with error recovery
    fn save_to_file_with_recovery(&self, metadata: &TrackMetadata) -> Result<(), OcypodeError> {
        use std::fs;
        use std::io::Write;

        let file_path = self.file_path_for_track(&metadata.track_id);
        let temp_path = file_path.with_extension("json.tmp");

        // Serialize metadata
        let content = serde_json::to_string_pretty(metadata).map_err(|e| {
            OcypodeError::TrackMetadataStorageError {
                reason: format!("Failed to serialize metadata: {}", e),
            }
        })?;

        // Write to temporary file first
        {
            let mut temp_file =
                fs::File::create(&temp_path).map_err(|e| OcypodeError::FileOperationError {
                    operation: "create_temp_file".to_string(),
                    reason: format!("Failed to create temporary file: {}", e),
                })?;

            temp_file.write_all(content.as_bytes()).map_err(|e| {
                OcypodeError::FileOperationError {
                    operation: "write_temp_file".to_string(),
                    reason: format!("Failed to write to temporary file: {}", e),
                }
            })?;

            temp_file
                .sync_all()
                .map_err(|e| OcypodeError::FileOperationError {
                    operation: "sync_temp_file".to_string(),
                    reason: format!("Failed to sync temporary file: {}", e),
                })?;
        }

        // Atomically move temporary file to final location
        fs::rename(&temp_path, &file_path).map_err(|e| {
            // Clean up temporary file on failure
            let _ = fs::remove_file(&temp_path);
            OcypodeError::FileOperationError {
                operation: "atomic_move".to_string(),
                reason: format!("Failed to move temporary file to final location: {}", e),
            }
        })?;

        Ok(())
    }

    /// Restore from backup if available
    fn restore_from_backup(&self, track_id: &str) -> Result<(), OcypodeError> {
        use std::fs;

        let file_path = self.file_path_for_track(track_id);
        let _backup_pattern = format!("{}.backup.*", file_path.display());

        // Find the most recent backup
        let parent_dir = file_path
            .parent()
            .ok_or_else(|| OcypodeError::FileOperationError {
                operation: "restore_backup".to_string(),
                reason: "Cannot determine parent directory".to_string(),
            })?;

        let entries = fs::read_dir(parent_dir).map_err(|e| OcypodeError::FileOperationError {
            operation: "restore_backup".to_string(),
            reason: format!("Cannot read directory: {}", e),
        })?;

        let mut backup_files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name
                        .starts_with(&format!("{}.backup.", Self::normalize_track_name(track_id)))
                    {
                        backup_files.push(path);
                    }
                }
            }
        }

        if let Some(latest_backup) = backup_files.into_iter().max() {
            fs::copy(&latest_backup, &file_path).map_err(|e| OcypodeError::FileOperationError {
                operation: "restore_backup".to_string(),
                reason: format!("Failed to restore from backup: {}", e),
            })?;

            log::info!("Restored from backup: {:?}", latest_backup);
            Ok(())
        } else {
            Err(OcypodeError::FileOperationError {
                operation: "restore_backup".to_string(),
                reason: "No backup files found".to_string(),
            })
        }
    }

    /// Clean up old backup files (keep only the 5 most recent)
    fn cleanup_old_backups(&self, track_id: &str) -> Result<(), OcypodeError> {
        use std::fs;

        let file_path = self.file_path_for_track(track_id);
        let parent_dir = file_path
            .parent()
            .ok_or_else(|| OcypodeError::FileOperationError {
                operation: "cleanup_backups".to_string(),
                reason: "Cannot determine parent directory".to_string(),
            })?;

        let entries = fs::read_dir(parent_dir).map_err(|e| OcypodeError::FileOperationError {
            operation: "cleanup_backups".to_string(),
            reason: format!("Cannot read directory: {}", e),
        })?;

        let mut backup_files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name
                        .starts_with(&format!("{}.backup.", Self::normalize_track_name(track_id)))
                    {
                        backup_files.push(path);
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        backup_files.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        backup_files.reverse();

        // Remove old backups (keep only 5 most recent)
        for old_backup in backup_files.into_iter().skip(5) {
            if let Err(e) = fs::remove_file(&old_backup) {
                log::warn!("Failed to remove old backup {:?}: {}", old_backup, e);
            } else {
                log::debug!("Removed old backup: {:?}", old_backup);
            }
        }

        Ok(())
    }
}

impl TrackMetadataStorage for FileBasedStorage {
    fn save_metadata(&mut self, metadata: &TrackMetadata) -> Result<(), OcypodeError> {
        use log::{error, info, warn};

        info!("Saving track metadata for: {}", metadata.track_name);

        // Comprehensive validation before saving
        if let Err(validation_error) = self.validate_metadata_for_save(metadata) {
            error!("Metadata validation failed: {}", validation_error);
            return Err(validation_error);
        }

        // Check available disk space before saving
        if let Err(space_error) = self.check_available_disk_space() {
            warn!("Disk space check failed: {}", space_error);
            // Continue with save but log the warning
        }

        // Create backup of existing metadata if it exists
        if let Err(backup_error) = self.create_backup_if_exists(&metadata.track_id) {
            warn!("Failed to create backup: {}", backup_error);
            // Continue with save but log the warning
        }

        // Save to file with error recovery
        match self.save_to_file_with_recovery(metadata) {
            Ok(()) => {
                info!(
                    "Successfully saved metadata for track: {}",
                    metadata.track_name
                );

                // Update cache only after successful save
                self.cache
                    .insert(metadata.track_id.clone(), metadata.clone());

                // Clean up old backups
                if let Err(cleanup_error) = self.cleanup_old_backups(&metadata.track_id) {
                    warn!("Failed to clean up old backups: {}", cleanup_error);
                    // Don't fail the save operation for cleanup errors
                }

                Ok(())
            }
            Err(save_error) => {
                error!(
                    "Failed to save metadata for track {}: {}",
                    metadata.track_name, save_error
                );

                // Attempt to restore from backup if save failed
                if let Err(restore_error) = self.restore_from_backup(&metadata.track_id) {
                    error!("Failed to restore from backup: {}", restore_error);
                }

                Err(save_error)
            }
        }
    }

    fn load_metadata(&self, track_name: &str) -> Result<Option<TrackMetadata>, OcypodeError> {
        use log::{debug, error, warn};

        debug!("Loading metadata for track: {}", track_name);

        // Input validation
        if track_name.is_empty() {
            return Err(OcypodeError::InvalidUserInput {
                field: "track_name".to_string(),
                reason: "Track name cannot be empty".to_string(),
            });
        }

        let normalized_name = Self::normalize_track_name(track_name);
        debug!("Normalized track name: {}", normalized_name);

        // Check cache first
        if let Some(metadata) = self.cache.get(&normalized_name) {
            debug!("Found metadata in cache for: {}", track_name);
            return Ok(Some(metadata.clone()));
        }

        // Load from file with comprehensive error handling
        match self.load_from_file_with_recovery(track_name) {
            Ok(Some(metadata)) => {
                debug!("Successfully loaded metadata from file for: {}", track_name);

                // Validate loaded metadata
                if let Err(validation_error) = self.validate_loaded_metadata(&metadata) {
                    warn!("Loaded metadata failed validation: {}", validation_error);
                    return Err(validation_error);
                }

                Ok(Some(metadata))
            }
            Ok(None) => {
                debug!("No metadata file found for: {}", track_name);
                Ok(None)
            }
            Err(e) => {
                error!("Failed to load metadata for {}: {}", track_name, e);
                Err(e)
            }
        }
    }

    fn list_available_tracks(&self) -> Result<Vec<String>, OcypodeError> {
        let mut tracks = Vec::new();

        let entries = fs::read_dir(&self.storage_path)
            .map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?;

        for entry in entries {
            let entry = entry.map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Try to load the metadata to get the actual track name
                    if let Ok(Some(metadata)) = self.load_from_file(stem) {
                        tracks.push(metadata.track_name);
                    } else {
                        // Fallback to filename if loading fails
                        tracks.push(stem.to_string());
                    }
                }
            }
        }

        tracks.sort();
        Ok(tracks)
    }

    fn delete_metadata(&mut self, track_name: &str) -> Result<(), OcypodeError> {
        let file_path = self.file_path_for_track(track_name);

        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| OcypodeError::ConfigIOError { source: e })?;
        }

        // Remove from cache
        let normalized_name = Self::normalize_track_name(track_name);
        self.cache.remove(&normalized_name);

        Ok(())
    }

    fn metadata_exists(&self, track_name: &str) -> Result<bool, OcypodeError> {
        let normalized_name = Self::normalize_track_name(track_name);

        // Check cache first
        if self.cache.contains_key(&normalized_name) {
            return Ok(true);
        }

        // Check file system
        let file_path = self.file_path_for_track(track_name);
        Ok(file_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track_metadata::types::{CornerAnnotation, CornerType};
    use tempfile::TempDir;

    fn create_test_metadata() -> TrackMetadata {
        let mut metadata = TrackMetadata::new(
            "Test Track".to_string(),
            "test_track".to_string(),
            "<svg><path d=\"M 100,100 L 200,200\" /></svg>".to_string(),
        );

        let corner = CornerAnnotation::new(1, 0.1, 0.2, CornerType::LeftHand).unwrap();
        metadata.add_corner(corner);

        metadata
    }

    #[test]
    fn test_file_based_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(storage.storage_path(), temp_dir.path());
    }

    #[test]
    fn test_track_name_normalization() {
        assert_eq!(
            FileBasedStorage::normalize_track_name("Silverstone GP"),
            "silverstone_gp"
        );
        assert_eq!(
            FileBasedStorage::normalize_track_name("Spa-Francorchamps"),
            "spa_francorchamps"
        );
        assert_eq!(
            FileBasedStorage::normalize_track_name("Circuit de Monaco"),
            "circuit_de_monaco"
        );
    }

    #[test]
    fn test_save_and_load_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let metadata = create_test_metadata();

        // Save metadata
        storage.save_metadata(&metadata).unwrap();

        // Load metadata
        let loaded = storage.load_metadata("test_track").unwrap();
        assert!(loaded.is_some());

        let loaded_metadata = loaded.unwrap();
        assert_eq!(loaded_metadata.track_name, metadata.track_name);
        assert_eq!(loaded_metadata.track_id, metadata.track_id);
        assert_eq!(loaded_metadata.svg_map, metadata.svg_map);
        assert_eq!(loaded_metadata.corners.len(), metadata.corners.len());
    }

    #[test]
    fn test_metadata_exists() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let metadata = create_test_metadata();

        // Initially doesn't exist
        assert!(!storage.metadata_exists("test_track").unwrap());

        // Save and check existence
        storage.save_metadata(&metadata).unwrap();
        assert!(storage.metadata_exists("test_track").unwrap());
    }

    #[test]
    fn test_list_available_tracks() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

        // Initially empty
        let tracks = storage.list_available_tracks().unwrap();
        assert_eq!(tracks.len(), 0);

        // Add some tracks
        let metadata1 = TrackMetadata::new(
            "Track A".to_string(),
            "track_a".to_string(),
            "<svg></svg>".to_string(),
        );
        let metadata2 = TrackMetadata::new(
            "Track B".to_string(),
            "track_b".to_string(),
            "<svg></svg>".to_string(),
        );

        storage.save_metadata(&metadata1).unwrap();
        storage.save_metadata(&metadata2).unwrap();

        let tracks = storage.list_available_tracks().unwrap();
        assert_eq!(tracks.len(), 2);
        assert!(tracks.contains(&"Track A".to_string()));
        assert!(tracks.contains(&"Track B".to_string()));
    }

    #[test]
    fn test_delete_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let metadata = create_test_metadata();

        // Save and verify existence
        storage.save_metadata(&metadata).unwrap();
        assert!(storage.metadata_exists("test_track").unwrap());

        // Delete and verify removal
        storage.delete_metadata("test_track").unwrap();
        assert!(!storage.metadata_exists("test_track").unwrap());

        // Load should return None
        let loaded = storage.load_metadata("test_track").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_cache_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = FileBasedStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let metadata = create_test_metadata();

        // Save metadata (should populate cache)
        storage.save_metadata(&metadata).unwrap();

        // Load should hit cache
        let loaded = storage.load_metadata("test_track").unwrap();
        assert!(loaded.is_some());

        // Clear cache and load again (should hit file system)
        storage.clear_cache();
        let loaded = storage.load_metadata("test_track").unwrap();
        assert!(loaded.is_some());
    }
}
