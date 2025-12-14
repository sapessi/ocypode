// SVG track map generator for converting telemetry position data into scalable vector graphics

use crate::errors::OcypodeError;
use crate::telemetry::TelemetryData;
use serde::{Deserialize, Serialize};

/// Configuration for SVG track map generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMapConfig {
    /// Canvas dimensions (width, height) in pixels
    pub canvas_size: (u32, u32),
    /// Stroke width for the track line
    pub stroke_width: f32,
    /// Scaling algorithm to use for coordinate normalization
    pub scaling_algorithm: ScalingAlgorithm,
    /// Margin around the track as percentage of canvas size
    pub margin_percentage: f32,
}

impl Default for TrackMapConfig {
    fn default() -> Self {
        Self {
            canvas_size: (800, 600),
            stroke_width: 3.0,
            scaling_algorithm: ScalingAlgorithm::AutoFit,
            margin_percentage: 0.1, // 10% margin
        }
    }
}

/// Scaling algorithms for coordinate normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingAlgorithm {
    /// Automatically fit track to canvas with uniform scaling
    AutoFit,
    /// Scale to fill canvas (may distort aspect ratio)
    FillCanvas,
    /// Use fixed scale factor
    FixedScale(f32),
}

/// Generator for creating SVG track maps from telemetry data
pub struct TrackMapGenerator {
    config: TrackMapConfig,
}

/// Represents a 2D coordinate point
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Bounding box for coordinate calculations
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl BoundingBox {
    pub fn new() -> Self {
        Self {
            min_x: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            min_y: f32::INFINITY,
            max_y: f32::NEG_INFINITY,
        }
    }

    pub fn update(&mut self, point: Point2D) {
        self.min_x = self.min_x.min(point.x);
        self.max_x = self.max_x.max(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_y = self.max_y.max(point.y);
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn center(&self) -> Point2D {
        Point2D::new(
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
}

/// Track position mapping that preserves the relationship between
/// SVG coordinates and track percentage values
#[derive(Debug, Clone)]
pub struct TrackPositionMapping {
    /// SVG coordinates mapped to track percentages
    pub position_map: Vec<(Point2D, f32)>,
}

impl TrackPositionMapping {
    pub fn new() -> Self {
        Self {
            position_map: Vec::new(),
        }
    }

    /// Add a mapping between SVG coordinate and track percentage
    pub fn add_mapping(&mut self, svg_point: Point2D, track_percentage: f32) {
        self.position_map.push((svg_point, track_percentage));
    }

    /// Find the track percentage for a given SVG coordinate (approximate)
    pub fn find_track_percentage(&self, svg_point: Point2D, tolerance: f32) -> Option<f32> {
        self.position_map
            .iter()
            .find(|(point, _)| {
                let distance =
                    ((point.x - svg_point.x).powi(2) + (point.y - svg_point.y).powi(2)).sqrt();
                distance <= tolerance
            })
            .map(|(_, percentage)| *percentage)
    }

    /// Find the SVG point for a given track percentage
    ///
    /// This method searches for the SVG coordinate that corresponds to the given track percentage.
    /// If an exact match is not found, it will interpolate between the two closest points.
    ///
    /// # Arguments
    ///
    /// * `track_percentage` - The track percentage to find (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// An `Option<Point2D>` containing the SVG coordinate if found, or None if the mapping is empty
    /// or the track percentage is outside the valid range.
    pub fn find_svg_point(&self, track_percentage: f32) -> Option<Point2D> {
        if self.position_map.is_empty() || track_percentage < 0.0 || track_percentage > 1.0 {
            return None;
        }

        // Handle edge case: only one point in mapping
        if self.position_map.len() == 1 {
            return Some(self.position_map[0].0);
        }

        // Sort the position map by track percentage for consistent lookup
        let mut sorted_map = self.position_map.clone();
        sorted_map.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Find exact match first
        if let Some((point, _)) = sorted_map
            .iter()
            .find(|(_, pct)| (*pct - track_percentage).abs() < f32::EPSILON)
        {
            return Some(*point);
        }

        // Find the two closest points for interpolation
        let mut before_point: Option<(Point2D, f32)> = None;
        let mut after_point: Option<(Point2D, f32)> = None;

        for &(point, pct) in &sorted_map {
            if pct <= track_percentage {
                before_point = Some((point, pct));
            } else if after_point.is_none() {
                after_point = Some((point, pct));
                break;
            }
        }

        match (before_point, after_point) {
            (Some((before_pt, before_pct)), Some((after_pt, after_pct))) => {
                // Interpolate between the two points
                let t = (track_percentage - before_pct) / (after_pct - before_pct);
                let interpolated_x = before_pt.x + t * (after_pt.x - before_pt.x);
                let interpolated_y = before_pt.y + t * (after_pt.y - before_pt.y);
                Some(Point2D::new(interpolated_x, interpolated_y))
            }
            (Some((point, _)), None) => {
                // Track percentage is beyond the last point, return the last point
                Some(point)
            }
            (None, Some((point, _))) => {
                // Track percentage is before the first point, return the first point
                Some(point)
            }
            (None, None) => {
                // This shouldn't happen if position_map is not empty, but handle gracefully
                None
            }
        }
    }
}

impl TrackMapGenerator {
    /// Create a new track map generator with default configuration
    pub fn new() -> Self {
        Self {
            config: TrackMapConfig::default(),
        }
    }

    /// Create a new track map generator with custom configuration
    pub fn with_config(config: TrackMapConfig) -> Self {
        Self { config }
    }

    /// Generate SVG track map from telemetry lap data with comprehensive error handling
    ///
    /// This method converts 3D world coordinates from telemetry data into a 2D SVG representation
    /// while preserving the relationship between track positions and track percentage values.
    ///
    /// # Arguments
    ///
    /// * `lap_data` - Vector of telemetry data points representing a complete lap
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - SVG string representation of the track
    /// - TrackPositionMapping for coordinate-to-percentage lookups
    ///
    /// # Requirements
    ///
    /// Validates Requirements 2.2, 2.3, 2.4:
    /// - 2.2: Generate SVG from telemetry position data
    /// - 2.3: Normalize coordinates to create properly scaled track representation  
    /// - 2.4: Preserve relationship between track positions and track percentage values
    pub fn generate_svg_from_lap(
        &self,
        lap_data: &[TelemetryData],
    ) -> Result<(String, TrackPositionMapping), OcypodeError> {
        use log::{debug, error, info, warn};

        info!(
            "Starting SVG generation from lap data with {} points",
            lap_data.len()
        );

        // Comprehensive input validation
        if let Err(validation_error) = self.validate_lap_data_for_svg(lap_data) {
            error!("Lap data validation failed: {}", validation_error);
            return Err(validation_error);
        }

        // Extract 3D world coordinates from telemetry data with error handling
        let world_coordinates = match self.extract_world_coordinates_with_validation(lap_data) {
            Ok(coords) => coords,
            Err(e) => {
                error!("Failed to extract world coordinates: {}", e);
                return Err(e);
            }
        };

        debug!(
            "Extracted {} world coordinate points",
            world_coordinates.len()
        );

        // Additional validation after coordinate extraction
        if world_coordinates.len() < 10 {
            warn!(
                "Very few coordinate points ({}), SVG quality may be poor",
                world_coordinates.len()
            );
        }

        // Convert 3D coordinates to 2D track representation
        let track_points = self.convert_to_2d_with_validation(&world_coordinates)?;
        debug!("Converted to {} 2D track points", track_points.len());

        // Validate track geometry
        if let Err(geometry_error) = self.validate_track_geometry(&track_points) {
            warn!("Track geometry validation failed: {}", geometry_error);
            // Continue with generation but log the warning
        }

        // Normalize and scale coordinates to fit canvas
        let (normalized_points, position_mapping) =
            self.normalize_coordinates_with_validation(&track_points, lap_data)?;
        debug!(
            "Normalized to {} points with position mapping",
            normalized_points.len()
        );

        // Generate SVG path from normalized coordinates
        let svg_content = match self.generate_svg_content_with_validation(&normalized_points) {
            Ok(svg) => svg,
            Err(e) => {
                error!("Failed to generate SVG content: {}", e);
                return Err(e);
            }
        };

        info!(
            "Successfully generated SVG with {} characters",
            svg_content.len()
        );
        Ok((svg_content, position_mapping))
    }

    /// Comprehensive validation of lap data for SVG generation
    fn validate_lap_data_for_svg(&self, lap_data: &[TelemetryData]) -> Result<(), OcypodeError> {
        if lap_data.is_empty() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Cannot generate SVG from empty lap data".to_string(),
            });
        }

        if lap_data.len() > 100_000 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!("Too many data points ({}, max 100,000)", lap_data.len()),
            });
        }

        // Check for minimum position data coverage
        let position_data_count = lap_data
            .iter()
            .filter(|t| {
                t.world_position_x.is_some()
                    && t.world_position_y.is_some()
                    && t.world_position_z.is_some()
            })
            .count();

        if position_data_count == 0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: "No position data found in telemetry".to_string(),
            });
        }

        let coverage_percentage = (position_data_count as f32 / lap_data.len() as f32) * 100.0;
        if coverage_percentage < 10.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Insufficient position data coverage ({:.1}%, minimum 10%)",
                    coverage_percentage
                ),
            });
        }

        if position_data_count < 3 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Insufficient position data points ({}, minimum 3)",
                    position_data_count
                ),
            });
        }

        Ok(())
    }

    /// Extract world coordinates from telemetry data
    fn extract_world_coordinates(
        &self,
        lap_data: &[TelemetryData],
    ) -> Result<Vec<(f32, f32, f32)>, OcypodeError> {
        let mut coordinates = Vec::new();

        for data_point in lap_data {
            if let (Some(x), Some(y), Some(z)) = (
                data_point.world_position_x,
                data_point.world_position_y,
                data_point.world_position_z,
            ) {
                coordinates.push((x, y, z));
            }
        }

        if coordinates.is_empty() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "No valid world position coordinates found in telemetry data"
                    .to_string(),
            });
        }

        Ok(coordinates)
    }

    /// Extract world coordinates with comprehensive validation and error recovery
    fn extract_world_coordinates_with_validation(
        &self,
        lap_data: &[TelemetryData],
    ) -> Result<Vec<(f32, f32, f32)>, OcypodeError> {
        use log::{debug, warn};

        let mut coordinates = Vec::new();
        let mut invalid_count = 0;
        let mut out_of_range_count = 0;

        for (index, data_point) in lap_data.iter().enumerate() {
            if let (Some(x), Some(y), Some(z)) = (
                data_point.world_position_x,
                data_point.world_position_y,
                data_point.world_position_z,
            ) {
                // Validate coordinate values
                if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                    invalid_count += 1;
                    debug!(
                        "Invalid coordinate at index {}: ({}, {}, {})",
                        index, x, y, z
                    );
                    continue;
                }

                // Check for reasonable coordinate ranges (within ±100km)
                if x.abs() > 100_000.0 || y.abs() > 100_000.0 || z.abs() > 100_000.0 {
                    out_of_range_count += 1;
                    debug!(
                        "Out-of-range coordinate at index {}: ({}, {}, {})",
                        index, x, y, z
                    );
                    continue;
                }

                coordinates.push((x, y, z));
            }
        }

        if coordinates.is_empty() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "No valid world position coordinates found in telemetry data".to_string(),
            });
        }

        // Log warnings about data quality
        if invalid_count > 0 {
            warn!(
                "Skipped {} invalid coordinates (NaN/Infinite values)",
                invalid_count
            );
        }

        if out_of_range_count > 0 {
            warn!(
                "Skipped {} out-of-range coordinates (>100km)",
                out_of_range_count
            );
        }

        let total_points = lap_data.len();
        let valid_percentage = (coordinates.len() as f32 / total_points as f32) * 100.0;

        if valid_percentage < 50.0 {
            warn!(
                "Low coordinate data quality: only {:.1}% of points are valid",
                valid_percentage
            );
        }

        debug!(
            "Extracted {} valid coordinates from {} total points ({:.1}%)",
            coordinates.len(),
            total_points,
            valid_percentage
        );

        Ok(coordinates)
    }

    /// Convert 3D coordinates to 2D with validation
    fn convert_to_2d_with_validation(
        &self,
        world_coordinates: &[(f32, f32, f32)],
    ) -> Result<Vec<Point2D>, OcypodeError> {
        use log::{debug, warn};

        if world_coordinates.is_empty() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Cannot convert empty coordinate list to 2D".to_string(),
            });
        }

        let track_points: Vec<Point2D> = world_coordinates
            .iter()
            .map(|(x, _y, z)| Point2D::new(*x, *z))
            .collect();

        debug!(
            "Converted {} 3D coordinates to 2D points",
            track_points.len()
        );

        // Validate the resulting 2D points
        let mut duplicate_count = 0;
        let mut previous_point: Option<Point2D> = None;

        for point in &track_points {
            if let Some(prev) = previous_point {
                let distance = ((point.x - prev.x).powi(2) + (point.y - prev.y).powi(2)).sqrt();
                if distance < 0.001 {
                    // Very close points (1mm)
                    duplicate_count += 1;
                }
            }
            previous_point = Some(*point);
        }

        if duplicate_count > track_points.len() / 2 {
            warn!(
                "High number of duplicate/very close points: {} out of {}",
                duplicate_count,
                track_points.len()
            );
        }

        Ok(track_points)
    }

    /// Validate track geometry for reasonable shape
    fn validate_track_geometry(&self, track_points: &[Point2D]) -> Result<(), OcypodeError> {
        use log::debug;

        if track_points.len() < 3 {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Insufficient points for track geometry validation".to_string(),
            });
        }

        // Calculate bounding box
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for point in track_points {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        let width = max_x - min_x;
        let height = max_y - min_y;

        debug!("Track geometry: width={:.2}, height={:.2}", width, height);

        // Check for degenerate geometry
        if width < 1.0 || height < 1.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Track geometry too small: {}x{} (minimum 1x1)",
                    width, height
                ),
            });
        }

        // Check for extremely elongated tracks
        let aspect_ratio = width.max(height) / width.min(height);
        if aspect_ratio > 20.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Track geometry too elongated: aspect ratio {:.1} (maximum 20)",
                    aspect_ratio
                ),
            });
        }

        // Check for reasonable track size (between 100m and 50km)
        let max_dimension = width.max(height);
        if max_dimension < 100.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!("Track too small: {:.1}m (minimum 100m)", max_dimension),
            });
        }

        if max_dimension > 50_000.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!("Track too large: {:.1}m (maximum 50km)", max_dimension),
            });
        }

        Ok(())
    }

    /// Convert 3D world coordinates to 2D track representation
    ///
    /// This uses a simple top-down projection, ignoring the Y (vertical) coordinate
    /// and using X and Z as the horizontal plane coordinates.
    fn convert_to_2d(&self, world_coordinates: &[(f32, f32, f32)]) -> Vec<Point2D> {
        world_coordinates
            .iter()
            .map(|(x, _y, z)| Point2D::new(*x, *z))
            .collect()
    }

    /// Normalize coordinates to fit within the canvas while preserving track percentage mapping
    fn normalize_coordinates(
        &self,
        track_points: &[Point2D],
        lap_data: &[TelemetryData],
    ) -> Result<(Vec<Point2D>, TrackPositionMapping), OcypodeError> {
        // Calculate bounding box of track points
        let mut bbox = BoundingBox::new();
        for point in track_points {
            bbox.update(*point);
        }

        // Calculate scaling factors based on configuration (for future use)
        let (_scale_x, _scale_y) = self.calculate_scaling_factors(&bbox)?;

        // Calculate canvas margins
        let margin_x = self.config.canvas_size.0 as f32 * self.config.margin_percentage;
        let margin_y = self.config.canvas_size.1 as f32 * self.config.margin_percentage;
        let usable_width = self.config.canvas_size.0 as f32 - 2.0 * margin_x;
        let usable_height = self.config.canvas_size.1 as f32 - 2.0 * margin_y;

        // Normalize and scale coordinates
        let mut normalized_points = Vec::new();
        let mut position_mapping = TrackPositionMapping::new();

        for (i, point) in track_points.iter().enumerate() {
            // Normalize to 0-1 range
            let normalized_x = (point.x - bbox.min_x) / bbox.width();
            let normalized_y = (point.y - bbox.min_y) / bbox.height();

            // Scale to canvas size with margins
            let canvas_x = margin_x + normalized_x * usable_width;
            let canvas_y = margin_y + normalized_y * usable_height;

            let svg_point = Point2D::new(canvas_x, canvas_y);
            normalized_points.push(svg_point);

            // Create position mapping if track percentage data is available
            if i < lap_data.len() {
                if let Some(track_pct) = lap_data[i]
                    .track_position_pct
                    .or(lap_data[i].lap_distance_pct)
                {
                    position_mapping.add_mapping(svg_point, track_pct);
                }
            }
        }

        Ok((normalized_points, position_mapping))
    }

    /// Normalize coordinates with comprehensive validation and error handling
    fn normalize_coordinates_with_validation(
        &self,
        track_points: &[Point2D],
        lap_data: &[TelemetryData],
    ) -> Result<(Vec<Point2D>, TrackPositionMapping), OcypodeError> {
        use log::{debug, warn};

        if track_points.is_empty() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Cannot normalize empty track points".to_string(),
            });
        }

        // Calculate bounding box of track points
        let mut bbox = BoundingBox::new();
        for point in track_points {
            bbox.update(*point);
        }

        debug!(
            "Track bounding box: ({:.2}, {:.2}) to ({:.2}, {:.2})",
            bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y
        );

        // Validate bounding box
        if !bbox.min_x.is_finite()
            || !bbox.max_x.is_finite()
            || !bbox.min_y.is_finite()
            || !bbox.max_y.is_finite()
        {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Invalid bounding box with non-finite coordinates".to_string(),
            });
        }

        if bbox.width() <= 0.0 || bbox.height() <= 0.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Invalid bounding box dimensions: {}x{}",
                    bbox.width(),
                    bbox.height()
                ),
            });
        }

        // Calculate scaling factors with validation
        let (scale_x, scale_y) = self.calculate_scaling_factors(&bbox)?;
        debug!(
            "Calculated scaling factors: x={:.4}, y={:.4}",
            scale_x, scale_y
        );

        // Validate canvas configuration
        if self.config.canvas_size.0 == 0 || self.config.canvas_size.1 == 0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Invalid canvas size: dimensions cannot be zero".to_string(),
            });
        }

        if self.config.margin_percentage < 0.0 || self.config.margin_percentage >= 0.5 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Invalid margin percentage: {} (must be 0.0-0.5)",
                    self.config.margin_percentage
                ),
            });
        }

        // Calculate canvas margins
        let margin_x = self.config.canvas_size.0 as f32 * self.config.margin_percentage;
        let margin_y = self.config.canvas_size.1 as f32 * self.config.margin_percentage;
        let usable_width = self.config.canvas_size.0 as f32 - 2.0 * margin_x;
        let usable_height = self.config.canvas_size.1 as f32 - 2.0 * margin_y;

        if usable_width <= 0.0 || usable_height <= 0.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Invalid usable canvas size: {}x{} (margins too large)",
                    usable_width, usable_height
                ),
            });
        }

        debug!(
            "Canvas: {}x{}, usable: {}x{}, margins: {}x{}",
            self.config.canvas_size.0,
            self.config.canvas_size.1,
            usable_width,
            usable_height,
            margin_x,
            margin_y
        );

        // Normalize and scale coordinates
        let mut normalized_points = Vec::new();
        let mut position_mapping = TrackPositionMapping::new();
        let mut mapping_count = 0;

        for (i, point) in track_points.iter().enumerate() {
            // Normalize to 0-1 range
            let normalized_x = (point.x - bbox.min_x) / bbox.width();
            let normalized_y = (point.y - bbox.min_y) / bbox.height();

            // Validate normalized coordinates
            if !normalized_x.is_finite() || !normalized_y.is_finite() {
                warn!(
                    "Skipping point {} with non-finite normalized coordinates",
                    i
                );
                continue;
            }

            // Scale to canvas size with margins
            let canvas_x = margin_x + normalized_x * usable_width;
            let canvas_y = margin_y + normalized_y * usable_height;

            // Validate final coordinates
            if !canvas_x.is_finite() || !canvas_y.is_finite() {
                warn!("Skipping point {} with non-finite canvas coordinates", i);
                continue;
            }

            let svg_point = Point2D::new(canvas_x, canvas_y);
            normalized_points.push(svg_point);

            // Create position mapping if track percentage data is available
            if i < lap_data.len() {
                if let Some(track_pct) = lap_data[i]
                    .track_position_pct
                    .or(lap_data[i].lap_distance_pct)
                {
                    if track_pct >= 0.0 && track_pct <= 1.0 {
                        position_mapping.add_mapping(svg_point, track_pct);
                        mapping_count += 1;
                    }
                }
            }
        }

        if normalized_points.is_empty() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "No valid points after normalization".to_string(),
            });
        }

        debug!(
            "Normalized {} points, created {} position mappings",
            normalized_points.len(),
            mapping_count
        );

        if mapping_count == 0 {
            warn!("No track percentage mappings created - track percentage data may be missing");
        }

        Ok((normalized_points, position_mapping))
    }

    /// Calculate scaling factors based on the configured scaling algorithm
    fn calculate_scaling_factors(&self, bbox: &BoundingBox) -> Result<(f32, f32), OcypodeError> {
        match self.config.scaling_algorithm {
            ScalingAlgorithm::AutoFit => {
                // Uniform scaling to fit within canvas while preserving aspect ratio
                let canvas_aspect =
                    self.config.canvas_size.0 as f32 / self.config.canvas_size.1 as f32;
                let track_aspect = bbox.width() / bbox.height();

                if track_aspect > canvas_aspect {
                    // Track is wider than canvas - scale based on width
                    let scale = self.config.canvas_size.0 as f32 / bbox.width();
                    Ok((scale, scale))
                } else {
                    // Track is taller than canvas - scale based on height
                    let scale = self.config.canvas_size.1 as f32 / bbox.height();
                    Ok((scale, scale))
                }
            }
            ScalingAlgorithm::FillCanvas => {
                // Scale to fill entire canvas (may distort aspect ratio)
                let scale_x = self.config.canvas_size.0 as f32 / bbox.width();
                let scale_y = self.config.canvas_size.1 as f32 / bbox.height();
                Ok((scale_x, scale_y))
            }
            ScalingAlgorithm::FixedScale(scale) => {
                // Use fixed scale factor
                Ok((scale, scale))
            }
        }
    }

    /// Generate SVG content from normalized coordinates
    fn generate_svg_content(&self, points: &[Point2D]) -> Result<String, OcypodeError> {
        if points.is_empty() {
            return Err(OcypodeError::TelemetryProducerError {
                description: "Cannot generate SVG from empty coordinate list".to_string(),
            });
        }

        let mut svg = String::new();

        // SVG header
        svg.push_str(&format!(
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <style>
      .track-line {{ stroke: #333; stroke-width: {}; fill: none; }}
      .corner-marker {{ fill: #ff0000; stroke: #000; stroke-width: 1; }}
    </style>
  </defs>"#,
            self.config.canvas_size.0, self.config.canvas_size.1, self.config.stroke_width
        ));

        // Generate SVG path
        svg.push_str("\n  <path class=\"track-line\" d=\"");

        // Start path with first point
        svg.push_str(&format!("M {:.2},{:.2}", points[0].x, points[0].y));

        // Add line segments for remaining points
        for point in points.iter().skip(1) {
            svg.push_str(&format!(" L {:.2},{:.2}", point.x, point.y));
        }

        // Close the path to complete the track loop
        svg.push_str(" Z");
        svg.push_str("\" />");

        // SVG footer
        svg.push_str("\n</svg>");

        Ok(svg)
    }

    /// Generate SVG content with comprehensive validation and error handling
    fn generate_svg_content_with_validation(
        &self,
        points: &[Point2D],
    ) -> Result<String, OcypodeError> {
        use log::{debug, warn};

        if points.is_empty() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Cannot generate SVG from empty coordinate list".to_string(),
            });
        }

        debug!("Generating SVG content from {} points", points.len());

        // Validate configuration
        if self.config.stroke_width <= 0.0 || self.config.stroke_width > 50.0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: format!(
                    "Invalid stroke width: {} (must be 0.1-50.0)",
                    self.config.stroke_width
                ),
            });
        }

        // Validate all points are within canvas bounds
        let mut out_of_bounds_count = 0;
        for (i, point) in points.iter().enumerate() {
            if point.x < 0.0
                || point.x > self.config.canvas_size.0 as f32
                || point.y < 0.0
                || point.y > self.config.canvas_size.1 as f32
            {
                out_of_bounds_count += 1;
                debug!(
                    "Point {} out of bounds: ({:.2}, {:.2})",
                    i, point.x, point.y
                );
            }
        }

        if out_of_bounds_count > 0 {
            warn!("{} points are outside canvas bounds", out_of_bounds_count);
        }

        let mut svg = String::with_capacity(1024 + points.len() * 20); // Pre-allocate capacity

        // SVG header with validation
        svg.push_str(&format!(
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">
  <defs>
    <style>
      .track-line {{ stroke: #333; stroke-width: {:.2}; fill: none; stroke-linecap: round; stroke-linejoin: round; }}
      .corner-marker {{ fill: #ff0000; stroke: #000; stroke-width: 1; }}
    </style>
  </defs>"#,
            self.config.canvas_size.0, self.config.canvas_size.1,
            self.config.canvas_size.0, self.config.canvas_size.1,
            self.config.stroke_width
        ));

        // Generate SVG path with error checking
        svg.push_str("\n  <path class=\"track-line\" d=\"");

        // Validate first point
        let first_point = &points[0];
        if !first_point.x.is_finite() || !first_point.y.is_finite() {
            return Err(OcypodeError::SvgGenerationError {
                reason: "First point has non-finite coordinates".to_string(),
            });
        }

        // Start path with first point
        svg.push_str(&format!("M {:.2},{:.2}", first_point.x, first_point.y));

        // Add line segments for remaining points with validation
        let mut valid_segments = 0;
        for (i, point) in points.iter().enumerate().skip(1) {
            if point.x.is_finite() && point.y.is_finite() {
                svg.push_str(&format!(" L {:.2},{:.2}", point.x, point.y));
                valid_segments += 1;
            } else {
                warn!(
                    "Skipping point {} with non-finite coordinates: ({}, {})",
                    i, point.x, point.y
                );
            }
        }

        if valid_segments == 0 {
            return Err(OcypodeError::SvgGenerationError {
                reason: "No valid line segments could be generated".to_string(),
            });
        }

        // Close the path to complete the track loop
        svg.push_str(" Z");
        svg.push_str("\" />");

        // Add metadata comment
        svg.push_str(&format!(
            "\n  <!-- Generated from {} points, {} valid segments -->",
            points.len(),
            valid_segments
        ));

        // SVG footer
        svg.push_str("\n</svg>");

        // Final validation
        if svg.len() > 10_000_000 {
            // 10MB limit
            return Err(OcypodeError::SvgGenerationError {
                reason: format!("Generated SVG too large: {} bytes (max 10MB)", svg.len()),
            });
        }

        // Validate SVG structure
        if !svg.starts_with("<svg") || !svg.ends_with("</svg>") {
            return Err(OcypodeError::SvgGenerationError {
                reason: "Generated SVG has invalid structure".to_string(),
            });
        }

        debug!(
            "Successfully generated SVG with {} characters, {} segments",
            svg.len(),
            valid_segments
        );
        Ok(svg)
    }

    /// Update generator configuration
    pub fn set_config(&mut self, config: TrackMapConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &TrackMapConfig {
        &self.config
    }
}

impl Default for TrackMapGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::GameSource;

    fn create_test_telemetry_data(
        positions: Vec<(f32, f32, f32)>,
        track_percentages: Vec<f32>,
    ) -> Vec<TelemetryData> {
        positions
            .into_iter()
            .zip(track_percentages.into_iter())
            .enumerate()
            .map(|(i, ((x, y, z), track_pct))| TelemetryData {
                point_no: i,
                timestamp_ms: (i as u128) * 100,
                game_source: GameSource::ACC,
                world_position_x: Some(x),
                world_position_y: Some(y),
                world_position_z: Some(z),
                track_position_pct: Some(track_pct),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn test_track_map_generator_creation() {
        let generator = TrackMapGenerator::new();
        assert_eq!(generator.config.canvas_size, (800, 600));
        assert_eq!(generator.config.stroke_width, 3.0);
        assert_eq!(generator.config.margin_percentage, 0.1);
    }

    #[test]
    fn test_track_map_generator_with_custom_config() {
        let config = TrackMapConfig {
            canvas_size: (1024, 768),
            stroke_width: 2.5,
            scaling_algorithm: ScalingAlgorithm::FillCanvas,
            margin_percentage: 0.05,
        };
        let generator = TrackMapGenerator::with_config(config.clone());
        assert_eq!(generator.config.canvas_size, config.canvas_size);
        assert_eq!(generator.config.stroke_width, config.stroke_width);
        assert_eq!(generator.config.margin_percentage, config.margin_percentage);
    }

    #[test]
    fn test_point2d_creation() {
        let point = Point2D::new(10.5, 20.3);
        assert_eq!(point.x, 10.5);
        assert_eq!(point.y, 20.3);
    }

    #[test]
    fn test_bounding_box_calculation() {
        let mut bbox = BoundingBox::new();

        bbox.update(Point2D::new(10.0, 20.0));
        bbox.update(Point2D::new(30.0, 5.0));
        bbox.update(Point2D::new(15.0, 25.0));

        assert_eq!(bbox.min_x, 10.0);
        assert_eq!(bbox.max_x, 30.0);
        assert_eq!(bbox.min_y, 5.0);
        assert_eq!(bbox.max_y, 25.0);
        assert_eq!(bbox.width(), 20.0);
        assert_eq!(bbox.height(), 20.0);

        let center = bbox.center();
        assert_eq!(center.x, 20.0);
        assert_eq!(center.y, 15.0);
    }

    #[test]
    fn test_track_position_mapping() {
        let mut mapping = TrackPositionMapping::new();

        mapping.add_mapping(Point2D::new(100.0, 200.0), 0.25);
        mapping.add_mapping(Point2D::new(300.0, 400.0), 0.75);

        // Test exact match
        assert_eq!(
            mapping.find_track_percentage(Point2D::new(100.0, 200.0), 1.0),
            Some(0.25)
        );

        // Test approximate match
        assert_eq!(
            mapping.find_track_percentage(Point2D::new(101.0, 201.0), 2.0),
            Some(0.25)
        );

        // Test no match
        assert_eq!(
            mapping.find_track_percentage(Point2D::new(500.0, 600.0), 1.0),
            None
        );
    }

    #[test]
    fn test_find_svg_point_exact_match() {
        let mut mapping = TrackPositionMapping::new();

        mapping.add_mapping(Point2D::new(100.0, 200.0), 0.25);
        mapping.add_mapping(Point2D::new(200.0, 300.0), 0.5);
        mapping.add_mapping(Point2D::new(300.0, 400.0), 0.75);

        // Test exact matches
        assert_eq!(
            mapping.find_svg_point(0.25),
            Some(Point2D::new(100.0, 200.0))
        );
        assert_eq!(
            mapping.find_svg_point(0.5),
            Some(Point2D::new(200.0, 300.0))
        );
        assert_eq!(
            mapping.find_svg_point(0.75),
            Some(Point2D::new(300.0, 400.0))
        );
    }

    #[test]
    fn test_find_svg_point_interpolation() {
        let mut mapping = TrackPositionMapping::new();

        mapping.add_mapping(Point2D::new(100.0, 200.0), 0.0);
        mapping.add_mapping(Point2D::new(300.0, 400.0), 1.0);

        // Test interpolation at midpoint
        let result = mapping.find_svg_point(0.5);
        assert!(result.is_some());
        let point = result.unwrap();
        assert!((point.x - 200.0).abs() < f32::EPSILON); // Should be halfway between 100 and 300
        assert!((point.y - 300.0).abs() < f32::EPSILON); // Should be halfway between 200 and 400

        // Test interpolation at quarter point
        let result = mapping.find_svg_point(0.25);
        assert!(result.is_some());
        let point = result.unwrap();
        assert!((point.x - 150.0).abs() < f32::EPSILON); // Should be 1/4 way from 100 to 300
        assert!((point.y - 250.0).abs() < f32::EPSILON); // Should be 1/4 way from 200 to 400
    }

    #[test]
    fn test_find_svg_point_edge_cases() {
        let mut mapping = TrackPositionMapping::new();

        // Test empty mapping
        assert_eq!(mapping.find_svg_point(0.5), None);

        // Test single point mapping
        mapping.add_mapping(Point2D::new(100.0, 200.0), 0.5);
        assert_eq!(
            mapping.find_svg_point(0.25),
            Some(Point2D::new(100.0, 200.0))
        );
        assert_eq!(
            mapping.find_svg_point(0.75),
            Some(Point2D::new(100.0, 200.0))
        );

        // Test invalid track percentages
        assert_eq!(mapping.find_svg_point(-0.1), None);
        assert_eq!(mapping.find_svg_point(1.1), None);
    }

    #[test]
    fn test_find_svg_point_boundary_conditions() {
        let mut mapping = TrackPositionMapping::new();

        mapping.add_mapping(Point2D::new(100.0, 200.0), 0.2);
        mapping.add_mapping(Point2D::new(200.0, 300.0), 0.5);
        mapping.add_mapping(Point2D::new(300.0, 400.0), 0.8);

        // Test track percentage before first point
        let result = mapping.find_svg_point(0.1);
        assert_eq!(result, Some(Point2D::new(100.0, 200.0))); // Should return first point

        // Test track percentage after last point
        let result = mapping.find_svg_point(0.9);
        assert_eq!(result, Some(Point2D::new(300.0, 400.0))); // Should return last point
    }

    #[test]
    fn test_find_svg_point_unsorted_input() {
        let mut mapping = TrackPositionMapping::new();

        // Add points in non-sequential order
        mapping.add_mapping(Point2D::new(300.0, 400.0), 0.75);
        mapping.add_mapping(Point2D::new(100.0, 200.0), 0.25);
        mapping.add_mapping(Point2D::new(200.0, 300.0), 0.5);

        // Should still work correctly due to internal sorting
        assert_eq!(
            mapping.find_svg_point(0.25),
            Some(Point2D::new(100.0, 200.0))
        );
        assert_eq!(
            mapping.find_svg_point(0.5),
            Some(Point2D::new(200.0, 300.0))
        );
        assert_eq!(
            mapping.find_svg_point(0.75),
            Some(Point2D::new(300.0, 400.0))
        );

        // Test interpolation with unsorted data
        let result = mapping.find_svg_point(0.375); // Midpoint between 0.25 and 0.5
        assert!(result.is_some());
        let point = result.unwrap();
        assert!((point.x - 150.0).abs() < f32::EPSILON);
        assert!((point.y - 250.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_extract_world_coordinates() {
        let generator = TrackMapGenerator::new();

        let telemetry_data = create_test_telemetry_data(
            vec![(10.0, 5.0, 20.0), (15.0, 6.0, 25.0), (20.0, 7.0, 30.0)],
            vec![0.0, 0.5, 1.0],
        );

        let coordinates = generator
            .extract_world_coordinates(&telemetry_data)
            .unwrap();
        assert_eq!(coordinates.len(), 3);
        assert_eq!(coordinates[0], (10.0, 5.0, 20.0));
        assert_eq!(coordinates[1], (15.0, 6.0, 25.0));
        assert_eq!(coordinates[2], (20.0, 7.0, 30.0));
    }

    #[test]
    fn test_extract_world_coordinates_with_missing_data() {
        let generator = TrackMapGenerator::new();

        let mut telemetry_data = create_test_telemetry_data(vec![(10.0, 5.0, 20.0)], vec![0.0]);

        // Remove position data from one point
        telemetry_data[0].world_position_x = None;

        let result = generator.extract_world_coordinates(&telemetry_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_2d() {
        let generator = TrackMapGenerator::new();

        let world_coords = vec![
            (10.0, 100.0, 20.0),
            (15.0, 200.0, 25.0),
            (20.0, 300.0, 30.0),
        ];
        let points_2d = generator.convert_to_2d(&world_coords);

        assert_eq!(points_2d.len(), 3);
        assert_eq!(points_2d[0], Point2D::new(10.0, 20.0)); // Y coordinate ignored
        assert_eq!(points_2d[1], Point2D::new(15.0, 25.0));
        assert_eq!(points_2d[2], Point2D::new(20.0, 30.0));
    }

    #[test]
    fn test_generate_svg_from_simple_lap() {
        let generator = TrackMapGenerator::new();

        // Create a simple rectangular track
        let telemetry_data = create_test_telemetry_data(
            vec![
                (0.0, 0.0, 0.0),
                (100.0, 0.0, 0.0),
                (100.0, 0.0, 100.0),
                (0.0, 0.0, 100.0),
            ],
            vec![0.0, 0.25, 0.5, 0.75],
        );

        let result = generator.generate_svg_from_lap(&telemetry_data);
        assert!(result.is_ok());

        let (svg, mapping) = result.unwrap();

        // Verify SVG contains expected elements
        assert!(svg.contains("<svg"));
        assert!(svg.contains("width=\"800\""));
        assert!(svg.contains("height=\"600\""));
        assert!(svg.contains("track-line"));
        assert!(svg.contains("</svg>"));

        // Verify position mapping was created
        assert_eq!(mapping.position_map.len(), 4);
    }

    #[test]
    fn test_generate_svg_from_empty_lap() {
        let generator = TrackMapGenerator::new();
        let empty_data: Vec<TelemetryData> = vec![];

        let result = generator.generate_svg_from_lap(&empty_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_svg_with_insufficient_points() {
        let generator = TrackMapGenerator::new();

        let telemetry_data = create_test_telemetry_data(
            vec![(0.0, 0.0, 0.0), (10.0, 0.0, 10.0)], // Only 2 points
            vec![0.0, 0.5],
        );

        let result = generator.generate_svg_from_lap(&telemetry_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_scaling_algorithm_auto_fit() {
        let generator = TrackMapGenerator::new();
        let bbox = BoundingBox {
            min_x: 0.0,
            max_x: 200.0,
            min_y: 0.0,
            max_y: 100.0,
        };

        let (scale_x, scale_y) = generator.calculate_scaling_factors(&bbox).unwrap();

        // Should use uniform scaling (same scale for both axes)
        assert_eq!(scale_x, scale_y);

        // Should scale based on the larger dimension (width in this case)
        assert_eq!(scale_x, 800.0 / 200.0); // canvas_width / bbox_width
    }

    #[test]
    fn test_scaling_algorithm_fill_canvas() {
        let config = TrackMapConfig {
            scaling_algorithm: ScalingAlgorithm::FillCanvas,
            ..Default::default()
        };
        let generator = TrackMapGenerator::with_config(config);

        let bbox = BoundingBox {
            min_x: 0.0,
            max_x: 200.0,
            min_y: 0.0,
            max_y: 100.0,
        };

        let (scale_x, scale_y) = generator.calculate_scaling_factors(&bbox).unwrap();

        // Should use different scales for each axis
        assert_eq!(scale_x, 800.0 / 200.0); // canvas_width / bbox_width
        assert_eq!(scale_y, 600.0 / 100.0); // canvas_height / bbox_height
        assert_ne!(scale_x, scale_y);
    }

    #[test]
    fn test_scaling_algorithm_fixed_scale() {
        let config = TrackMapConfig {
            scaling_algorithm: ScalingAlgorithm::FixedScale(2.5),
            ..Default::default()
        };
        let generator = TrackMapGenerator::with_config(config);

        let bbox = BoundingBox {
            min_x: 0.0,
            max_x: 200.0,
            min_y: 0.0,
            max_y: 100.0,
        };

        let (scale_x, scale_y) = generator.calculate_scaling_factors(&bbox).unwrap();

        // Should use the fixed scale factor
        assert_eq!(scale_x, 2.5);
        assert_eq!(scale_y, 2.5);
    }

    #[test]
    fn test_svg_content_generation() {
        let generator = TrackMapGenerator::new();

        let points = vec![
            Point2D::new(100.0, 200.0),
            Point2D::new(300.0, 200.0),
            Point2D::new(300.0, 400.0),
            Point2D::new(100.0, 400.0),
        ];

        let svg = generator.generate_svg_content(&points).unwrap();

        // Verify SVG structure
        assert!(svg.contains("<svg width=\"800\" height=\"600\""));
        assert!(svg.contains("track-line"));
        assert!(svg.contains("stroke-width: 3"));
        assert!(svg.contains("M 100.00,200.00"));
        assert!(svg.contains("L 300.00,200.00"));
        assert!(svg.contains("L 300.00,400.00"));
        assert!(svg.contains("L 100.00,400.00"));
        assert!(svg.contains("Z")); // Path should be closed
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_svg_content_generation_empty_points() {
        let generator = TrackMapGenerator::new();
        let empty_points: Vec<Point2D> = vec![];

        let result = generator.generate_svg_content(&empty_points);
        assert!(result.is_err());
    }
}
