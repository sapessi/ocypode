// Corner annotation UI tool for interactive corner placement and management

use crate::track_metadata::{CornerAnnotation, CornerType, TrackMetadata, TrackPositionMapping};
use egui::{Color32, Pos2, Rect, Response, Sense, Ui, Vec2};

/// Simple trait extension for string title case
trait ToTitleCase {
    fn to_title_case(&self) -> String;
}

impl ToTitleCase for str {
    fn to_title_case(&self) -> String {
        let mut chars = self.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => {
                first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
            }
        }
    }
}

/// UI tool for interactive corner annotation on SVG track maps
pub struct CornerAnnotationTool {
    /// Current track metadata being edited
    pub current_track_metadata: Option<TrackMetadata>,
    /// Currently selected corner for editing
    pub selected_corner: Option<u32>,
    /// Track position mapping for coordinate conversion
    pub track_position_mapping: Option<TrackPositionMapping>,
    /// Dialog state for corner input
    pub show_corner_input_dialog: bool,
    /// Temporary corner data for dialog input
    pub temp_corner_number: String,
    pub temp_corner_type: CornerType,
    pub temp_corner_start: String,
    pub temp_corner_end: String,
    pub temp_corner_description: String,
    /// Click position for new corner placement
    pub pending_corner_position: Option<Pos2>,
    /// Error message for validation
    pub validation_error: Option<String>,
}

impl Default for CornerAnnotationTool {
    fn default() -> Self {
        Self {
            current_track_metadata: None,
            selected_corner: None,
            track_position_mapping: None,
            show_corner_input_dialog: false,
            temp_corner_number: String::new(),
            temp_corner_type: CornerType::LeftHand,
            temp_corner_start: String::new(),
            temp_corner_end: String::new(),
            temp_corner_description: String::new(),
            pending_corner_position: None,
            validation_error: None,
        }
    }
}

impl CornerAnnotationTool {
    /// Create a new corner annotation tool
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current track metadata for editing
    pub fn set_track_metadata(
        &mut self,
        metadata: TrackMetadata,
        position_mapping: TrackPositionMapping,
    ) {
        self.current_track_metadata = Some(metadata);
        self.track_position_mapping = Some(position_mapping);
        self.selected_corner = None;
    }

    /// Add a new corner annotation
    pub fn add_corner(
        &mut self,
        _track_percentage: f32,
        corner_number: u32,
        corner_type: CornerType,
        start: f32,
        end: f32,
    ) -> Result<(), String> {
        if let Some(ref mut metadata) = self.current_track_metadata {
            // Validate corner number uniqueness
            if metadata
                .corners
                .iter()
                .any(|c| c.corner_number == corner_number)
            {
                return Err(format!("Corner number {} already exists", corner_number));
            }

            // Create new corner annotation
            let corner = CornerAnnotation::new(corner_number, start, end, corner_type)?;
            metadata.add_corner(corner);

            // Validate all corners after addition
            metadata.validate_corners()?;

            Ok(())
        } else {
            Err("No track metadata loaded".to_string())
        }
    }

    /// Update an existing corner's range
    pub fn update_corner_range(
        &mut self,
        corner_number: u32,
        start: f32,
        end: f32,
    ) -> Result<(), String> {
        if let Some(ref mut metadata) = self.current_track_metadata {
            // Find and update the corner
            if let Some(corner) = metadata
                .corners
                .iter_mut()
                .find(|c| c.corner_number == corner_number)
            {
                // Validate new range
                if !(0.0..=1.0).contains(&start) || !(0.0..=1.0).contains(&end) {
                    return Err("Track percentages must be between 0.0 and 1.0".to_string());
                }
                if start >= end {
                    return Err("Start percentage must be less than end percentage".to_string());
                }

                corner.track_percentage_start = start;
                corner.track_percentage_end = end;
                metadata.touch();

                // Validate all corners after update
                metadata.validate_corners()?;

                Ok(())
            } else {
                Err(format!("Corner {} not found", corner_number))
            }
        } else {
            Err("No track metadata loaded".to_string())
        }
    }

    /// Delete a corner annotation
    pub fn delete_corner(&mut self, corner_number: u32) -> Result<(), String> {
        if let Some(ref mut metadata) = self.current_track_metadata {
            if metadata.remove_corner(corner_number) {
                if self.selected_corner == Some(corner_number) {
                    self.selected_corner = None;
                }
                Ok(())
            } else {
                Err(format!("Corner {} not found", corner_number))
            }
        } else {
            Err("No track metadata loaded".to_string())
        }
    }

    /// Validate all corner ranges for uniqueness and overlap
    pub fn validate_corner_ranges(&self) -> Result<(), String> {
        if let Some(ref metadata) = self.current_track_metadata {
            metadata.validate_corners()
        } else {
            Err("No track metadata loaded".to_string())
        }
    }

    /// Convert screen position to track percentage using position mapping
    pub fn screen_pos_to_track_percentage(&self, screen_pos: Pos2, svg_rect: Rect) -> Option<f32> {
        if let Some(ref mapping) = self.track_position_mapping {
            // Convert screen position to SVG coordinates
            let svg_x = (screen_pos.x - svg_rect.min.x) / svg_rect.width();
            let svg_y = (screen_pos.y - svg_rect.min.y) / svg_rect.height();

            // Find closest point in the position mapping
            let mut closest_distance = f32::INFINITY;
            let mut closest_percentage = 0.0;

            for (pos, percentage) in &mapping.position_map {
                let dx = svg_x - pos.x;
                let dy = svg_y - pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < closest_distance {
                    closest_distance = distance;
                    closest_percentage = *percentage;
                }
            }

            // Only return if we're reasonably close to the track
            if closest_distance < 0.05 {
                // 5% of SVG dimensions
                Some(closest_percentage)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Show the SVG track map with interactive corner annotations
    pub fn show_interactive_svg(&mut self, ui: &mut Ui, _svg_content: &str) -> Response {
        let available_size = ui.available_size();
        let svg_size = Vec2::new(available_size.x.min(400.0), available_size.y.min(300.0));

        let (rect, response) = ui.allocate_exact_size(svg_size, Sense::click());

        // Draw SVG background (simplified representation)
        ui.painter().rect_filled(rect, 5.0, Color32::from_gray(40));

        // Draw track path (simplified - in a real implementation, this would render the actual SVG)
        let center = rect.center();
        let radius = rect.width().min(rect.height()) * 0.3;

        // Draw a simple oval track for visualization
        let mut points = Vec::new();
        for i in 0..32 {
            let angle = (i as f32 / 32.0) * 2.0 * std::f32::consts::PI;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * 0.6 * angle.sin();
            points.push(Pos2::new(x, y));
        }

        // Draw track outline
        for i in 0..points.len() {
            let start = points[i];
            let end = points[(i + 1) % points.len()];
            ui.painter()
                .line_segment([start, end], egui::Stroke::new(3.0, Color32::LIGHT_GRAY));
        }

        // Draw existing corner markers
        if let Some(ref metadata) = self.current_track_metadata {
            for corner in &metadata.corners {
                // Calculate position on track for corner marker
                let mid_percentage =
                    (corner.track_percentage_start + corner.track_percentage_end) / 2.0;
                let angle = mid_percentage * 2.0 * std::f32::consts::PI;
                let marker_x = center.x + radius * angle.cos();
                let marker_y = center.y + radius * 0.6 * angle.sin();
                let marker_pos = Pos2::new(marker_x, marker_y);

                // Choose color based on selection
                let color = if self.selected_corner == Some(corner.corner_number) {
                    Color32::YELLOW
                } else {
                    Color32::RED
                };

                // Draw corner marker
                ui.painter().circle_filled(marker_pos, 8.0, color);
                ui.painter()
                    .circle_stroke(marker_pos, 8.0, egui::Stroke::new(2.0, Color32::BLACK));

                // Draw corner number
                ui.painter().text(
                    marker_pos + Vec2::new(12.0, -8.0),
                    egui::Align2::LEFT_TOP,
                    corner.corner_number.to_string(),
                    egui::FontId::default(),
                    Color32::WHITE,
                );
            }
        }

        // Handle clicks for corner placement
        if response.clicked()
            && let Some(click_pos) = response.interact_pointer_pos()
        {
            // Check if we clicked on an existing corner
            let mut clicked_corner = None;

            if let Some(ref metadata) = self.current_track_metadata {
                for corner in &metadata.corners {
                    let mid_percentage =
                        (corner.track_percentage_start + corner.track_percentage_end) / 2.0;
                    let angle = mid_percentage * 2.0 * std::f32::consts::PI;
                    let marker_x = center.x + radius * angle.cos();
                    let marker_y = center.y + radius * 0.6 * angle.sin();
                    let marker_pos = Pos2::new(marker_x, marker_y);

                    let distance = (click_pos - marker_pos).length();
                    if distance <= 12.0 {
                        // Click tolerance
                        clicked_corner = Some(corner.corner_number);
                        break;
                    }
                }
            }

            if let Some(corner_number) = clicked_corner {
                // Select existing corner
                self.selected_corner = Some(corner_number);
            } else {
                // Start new corner placement
                self.pending_corner_position = Some(click_pos);
                self.show_corner_input_dialog = true;
                self.reset_dialog_fields();
            }
        }

        response
    }

    /// Show the corner input dialog
    pub fn show_corner_input_dialog(&mut self, ui: &mut Ui) {
        if !self.show_corner_input_dialog {
            return;
        }

        egui::Window::new("Add Corner")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    // Corner number input
                    ui.horizontal(|ui| {
                        ui.label("Corner Number:");
                        ui.text_edit_singleline(&mut self.temp_corner_number);
                    });

                    // Corner type selection
                    ui.horizontal(|ui| {
                        ui.label("Corner Type:");
                        egui::ComboBox::from_label("")
                            .selected_text(self.temp_corner_type.description())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.temp_corner_type,
                                    CornerType::LeftHand,
                                    "Left-hand turn",
                                );
                                ui.selectable_value(
                                    &mut self.temp_corner_type,
                                    CornerType::RightHand,
                                    "Right-hand turn",
                                );
                                ui.selectable_value(
                                    &mut self.temp_corner_type,
                                    CornerType::Chicane,
                                    "Chicane",
                                );
                                ui.selectable_value(
                                    &mut self.temp_corner_type,
                                    CornerType::Hairpin,
                                    "Hairpin turn",
                                );
                            });
                    });

                    // Track percentage range inputs
                    ui.horizontal(|ui| {
                        ui.label("Start %:");
                        ui.text_edit_singleline(&mut self.temp_corner_start);
                    });

                    ui.horizontal(|ui| {
                        ui.label("End %:");
                        ui.text_edit_singleline(&mut self.temp_corner_end);
                    });

                    // Optional description
                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut self.temp_corner_description);
                    });

                    // Show validation error if any
                    if let Some(ref error) = self.validation_error {
                        ui.colored_label(Color32::RED, error);
                    }

                    ui.separator();

                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button("Add Corner").clicked() {
                            self.try_add_corner_from_dialog();
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_corner_input_dialog = false;
                            self.pending_corner_position = None;
                            self.validation_error = None;
                        }
                    });
                });
            });
    }

    /// Try to add corner from dialog input with comprehensive validation and user-friendly error messages
    fn try_add_corner_from_dialog(&mut self) {
        use log::{debug, warn};

        self.validation_error = None;
        debug!("Attempting to add corner from dialog input");

        // Comprehensive input validation with user-friendly error messages
        let corner_number = match self.validate_corner_number_input() {
            Ok(n) => n,
            Err(e) => {
                self.validation_error = Some(e);
                return;
            }
        };

        let start_pct = match self.validate_percentage_input(&self.temp_corner_start, "start") {
            Ok(f) => f,
            Err(e) => {
                self.validation_error = Some(e);
                return;
            }
        };

        let end_pct = match self.validate_percentage_input(&self.temp_corner_end, "end") {
            Ok(f) => f,
            Err(e) => {
                self.validation_error = Some(e);
                return;
            }
        };

        // Validate percentage range relationship
        if let Err(e) = self.validate_percentage_range(start_pct, end_pct) {
            self.validation_error = Some(e);
            return;
        }

        // Validate description if provided
        if let Err(e) = self.validate_description_input() {
            self.validation_error = Some(e);
            return;
        }

        debug!(
            "All inputs validated successfully: corner={}, start={:.3}, end={:.3}",
            corner_number, start_pct, end_pct
        );

        // Try to add the corner with enhanced error handling
        match self.add_corner_with_recovery(
            start_pct,
            corner_number,
            self.temp_corner_type.clone(),
            start_pct,
            end_pct,
        ) {
            Ok(()) => {
                log::info!(
                    "Successfully added corner {} ({:.3}-{:.3})",
                    corner_number,
                    start_pct,
                    end_pct
                );
                self.show_corner_input_dialog = false;
                self.pending_corner_position = None;
                self.validation_error = None;
                self.reset_dialog_fields();
            }
            Err(e) => {
                warn!("Failed to add corner: {}", e);
                self.validation_error = Some(self.create_user_friendly_error_message(&e));
            }
        }
    }

    /// Validate corner number input with user-friendly error messages
    fn validate_corner_number_input(&self) -> Result<u32, String> {
        if self.temp_corner_number.trim().is_empty() {
            return Err("Please enter a corner number".to_string());
        }

        match self.temp_corner_number.trim().parse::<u32>() {
            Ok(0) => Err("Corner number must be greater than 0".to_string()),
            Ok(n) if n > 99 => Err("Corner number must be 99 or less".to_string()),
            Ok(n) => Ok(n),
            Err(_) => {
                if self.temp_corner_number.contains('.') {
                    Err("Corner number must be a whole number (no decimals)".to_string())
                } else if self.temp_corner_number.contains('-') {
                    Err("Corner number must be positive".to_string())
                } else {
                    Err("Please enter a valid corner number (1-99)".to_string())
                }
            }
        }
    }

    /// Validate percentage input with user-friendly error messages
    fn validate_percentage_input(&self, input: &str, field_name: &str) -> Result<f32, String> {
        if input.trim().is_empty() {
            return Err(format!("Please enter a {} percentage", field_name));
        }

        match input.trim().parse::<f32>() {
            Ok(f) if f < 0.0 => Err(format!(
                "{} percentage cannot be negative",
                field_name.to_title_case()
            )),
            Ok(f) if f > 1.0 => Err(format!(
                "{} percentage cannot be greater than 1.0 (100%)",
                field_name.to_title_case()
            )),
            Ok(f) if !f.is_finite() => {
                Err(format!("Please enter a valid {} percentage", field_name))
            }
            Ok(f) => Ok(f),
            Err(_) => Err(format!(
                "Please enter a valid decimal number for {} percentage (e.g., 0.25 for 25%)",
                field_name
            )),
        }
    }

    /// Validate percentage range relationship
    fn validate_percentage_range(&self, start_pct: f32, end_pct: f32) -> Result<(), String> {
        if start_pct >= end_pct {
            let diff = end_pct - start_pct;
            if diff.abs() < 0.001 {
                return Err("Start and end percentages cannot be the same".to_string());
            } else {
                return Err("Start percentage must be less than end percentage".to_string());
            }
        }

        let range_size = end_pct - start_pct;
        if range_size < 0.001 {
            return Err("Corner range is too small (minimum 0.1%)".to_string());
        }

        if range_size > 0.5 {
            return Err("Corner range is too large (maximum 50% of track)".to_string());
        }

        Ok(())
    }

    /// Validate description input
    fn validate_description_input(&self) -> Result<(), String> {
        if self.temp_corner_description.len() > 200 {
            return Err(format!(
                "Description too long ({} characters, max 200)",
                self.temp_corner_description.len()
            ));
        }

        // Check for potentially problematic characters
        if self.temp_corner_description.contains('\0') {
            return Err("Description contains invalid characters".to_string());
        }

        Ok(())
    }

    /// Add corner with recovery mechanisms
    fn add_corner_with_recovery(
        &mut self,
        _track_percentage: f32,
        corner_number: u32,
        corner_type: CornerType,
        start: f32,
        end: f32,
    ) -> Result<(), String> {
        // First attempt: normal add
        match self.add_corner(
            _track_percentage,
            corner_number,
            corner_type.clone(),
            start,
            end,
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::debug!("First add attempt failed: {}", e);

                // Check if it's a duplicate corner number error
                if e.contains("already exists") {
                    // Suggest next available corner number
                    if let Some(suggested_number) = self.suggest_next_corner_number() {
                        return Err(format!(
                            "Corner {} already exists. Try corner number {} instead.",
                            corner_number, suggested_number
                        ));
                    }
                }

                // Check if it's an overlap error
                if e.contains("overlaps")
                    && let Some(suggestion) = self.suggest_non_overlapping_range(start, end)
                {
                    return Err(format!(
                        "Corner range overlaps with existing corner. Try range {:.3}-{:.3} instead.",
                        suggestion.0, suggestion.1
                    ));
                }

                // Return original error if no recovery suggestions available
                Err(e)
            }
        }
    }

    /// Suggest next available corner number
    fn suggest_next_corner_number(&self) -> Option<u32> {
        if let Some(ref metadata) = self.current_track_metadata {
            let existing_numbers: std::collections::HashSet<u32> =
                metadata.corners.iter().map(|c| c.corner_number).collect();

            for i in 1..=99 {
                if !existing_numbers.contains(&i) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Suggest non-overlapping range
    fn suggest_non_overlapping_range(&self, start: f32, end: f32) -> Option<(f32, f32)> {
        if let Some(ref metadata) = self.current_track_metadata {
            let range_size = end - start;

            // Try to find a gap in existing corners
            let mut occupied_ranges: Vec<(f32, f32)> = metadata
                .corners
                .iter()
                .map(|c| (c.track_percentage_start, c.track_percentage_end))
                .collect();

            occupied_ranges
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            // Look for gaps between existing corners
            let mut current_pos = 0.0;
            for (occupied_start, occupied_end) in occupied_ranges {
                if occupied_start - current_pos >= range_size {
                    // Found a gap
                    let suggested_start = current_pos + 0.01; // Small buffer
                    let suggested_end = suggested_start + range_size;
                    if suggested_end < occupied_start - 0.01 {
                        return Some((suggested_start, suggested_end));
                    }
                }
                current_pos = occupied_end;
            }

            // Check if there's space at the end
            if 1.0 - current_pos >= range_size {
                let suggested_start = current_pos + 0.01;
                let suggested_end = suggested_start + range_size;
                if suggested_end <= 1.0 {
                    return Some((suggested_start, suggested_end));
                }
            }
        }
        None
    }

    /// Create user-friendly error message from technical error
    fn create_user_friendly_error_message(&self, error: &str) -> String {
        if error.contains("already exists") {
            "This corner number is already used. Please choose a different number.".to_string()
        } else if error.contains("overlaps") {
            "This corner range overlaps with an existing corner. Please adjust the start and end percentages.".to_string()
        } else if error.contains("Track percentages must be between") {
            "Track percentages must be between 0.0 and 1.0 (0% to 100%).".to_string()
        } else if error.contains("No track metadata loaded") {
            "No track is currently loaded. Please generate an SVG track map first.".to_string()
        } else {
            format!("Error: {}", error)
        }
    }

    /// Reset dialog input fields
    fn reset_dialog_fields(&mut self) {
        self.temp_corner_number.clear();
        self.temp_corner_type = CornerType::LeftHand;
        self.temp_corner_start.clear();
        self.temp_corner_end.clear();
        self.temp_corner_description.clear();
        self.validation_error = None;
    }

    /// Show corner management panel
    pub fn show_corner_management_panel(&mut self, ui: &mut Ui) {
        ui.heading("Corner Management");
        ui.separator();

        if let Some(ref metadata) = self.current_track_metadata {
            if metadata.corners.is_empty() {
                ui.label("No corners defined. Click on the track map to add corners.");
            } else {
                ui.label(format!("Track: {}", metadata.track_name));
                ui.label(format!("Corners: {}", metadata.corners.len()));
                ui.separator();

                // List existing corners
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for corner in &metadata.corners {
                            let is_selected = self.selected_corner == Some(corner.corner_number);

                            ui.group(|ui| {
                                let response = ui.selectable_label(
                                    is_selected,
                                    format!(
                                        "Corner {} ({})",
                                        corner.corner_number,
                                        corner.corner_type.description()
                                    ),
                                );

                                if response.clicked() {
                                    self.selected_corner = Some(corner.corner_number);
                                }

                                ui.label(format!(
                                    "Range: {:.3} - {:.3}",
                                    corner.track_percentage_start, corner.track_percentage_end
                                ));

                                if let Some(ref desc) = corner.description {
                                    ui.label(format!("Description: {}", desc));
                                }
                            });
                        }
                    });

                ui.separator();

                // Corner actions
                if let Some(selected) = self.selected_corner {
                    ui.horizontal(|ui| {
                        if ui.button("Delete Corner").clicked()
                            && let Err(e) = self.delete_corner(selected)
                        {
                            log::error!("Failed to delete corner: {}", e);
                        }
                    });
                }
            }

            ui.separator();

            // Validation status
            match self.validate_corner_ranges() {
                Ok(()) => {
                    ui.colored_label(Color32::GREEN, "✓ All corners valid");
                }
                Err(e) => {
                    ui.colored_label(Color32::RED, format!("⚠ Validation error: {}", e));
                }
            }
        } else {
            ui.label("No track metadata loaded");
        }
    }

    /// Get the current track metadata
    pub fn get_track_metadata(&self) -> Option<&TrackMetadata> {
        self.current_track_metadata.as_ref()
    }

    /// Get mutable reference to track metadata
    pub fn get_track_metadata_mut(&mut self) -> Option<&mut TrackMetadata> {
        self.current_track_metadata.as_mut()
    }
}
