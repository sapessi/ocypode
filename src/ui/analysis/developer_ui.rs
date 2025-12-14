use egui::{Color32, RichText, Ui};

use crate::{
    track_metadata::{
        FileBasedStorage, TrackMapConfig, TrackMapGenerator, TrackMetadata, TrackMetadataStorage,
        TrackPositionMapping,
    },
    ui::CornerAnnotationTool,
};

use super::{Lap, Session};

pub struct DeveloperUiState {
    // Developer mode specific fields
    pub svg_preview: Option<String>,
    pub track_position_mapping: Option<TrackPositionMapping>,
    pub show_corner_annotation_dialog: bool,
    pub generated_metadata: Option<TrackMetadata>,
    pub corner_annotation_tool: CornerAnnotationTool,
    // Storage integration
    pub metadata_storage: Option<FileBasedStorage>,
    pub show_save_confirmation: bool,
    pub show_load_dialog: bool,
    pub available_tracks: Vec<String>,
    pub save_status_message: Option<String>,
    pub load_status_message: Option<String>,
}

impl Default for DeveloperUiState {
    fn default() -> Self {
        // Initialize storage for developer mode
        let metadata_storage = match FileBasedStorage::new_default() {
            Ok(storage) => Some(storage),
            Err(e) => {
                log::error!("Failed to initialize metadata storage: {}", e);
                None
            }
        };

        Self {
            svg_preview: None,
            track_position_mapping: None,
            show_corner_annotation_dialog: false,
            generated_metadata: None,
            corner_annotation_tool: CornerAnnotationTool::new(),
            metadata_storage,
            show_save_confirmation: false,
            show_load_dialog: false,
            available_tracks: Vec::new(),
            save_status_message: None,
            load_status_message: None,
        }
    }
}

impl DeveloperUiState {
    pub fn show_developer_mode_panel(
        &mut self,
        ui: &mut Ui,
        session: &Session,
        selected_lap: &str,
    ) {
        ui.heading(RichText::new("Track Metadata Manager").color(Color32::WHITE));
        ui.separator();

        // Session information display - make it collapsible to save space
        egui::CollapsingHeader::new(
            RichText::new("Session Information")
                .color(Color32::WHITE)
                .strong(),
        )
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Track:").color(Color32::GRAY));
                ui.label(RichText::new(&session.info.track_name).color(Color32::WHITE));
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Total Laps:").color(Color32::GRAY));
                ui.label(RichText::new(session.laps.len().to_string()).color(Color32::WHITE));
            });
        });

        ui.add_space(5.0);

        // Enhanced lap selector with performance metrics - make it collapsible
        self.show_lap_selector(ui, session, selected_lap);

        ui.add_space(5.0);

        // SVG Preview Panel - make it collapsible
        self.show_svg_preview_panel(ui, session, selected_lap);

        ui.add_space(5.0);

        // Corner annotation interface - make it collapsible
        self.show_corner_annotation_panel(ui, session, selected_lap);

        ui.add_space(5.0);

        // Track metadata creation tools - make it collapsible
        self.show_metadata_tools_panel(ui);

        ui.add_space(5.0);

        // Status section with detailed information - make it collapsible
        self.show_status_panel(ui, session, selected_lap);
    }

    fn show_lap_selector(&mut self, ui: &mut Ui, session: &Session, selected_lap: &str) {
        egui::CollapsingHeader::new(
            RichText::new("Lap Selection")
                .color(Color32::WHITE)
                .strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    for (lap_index, lap) in session.laps.iter().enumerate() {
                        let is_selected = selected_lap == lap_index.to_string();

                        // Calculate lap metrics
                        let lap_time = if !lap.telemetry.is_empty() {
                            lap.telemetry.last().unwrap().last_lap_time_s.unwrap_or(0.0)
                        } else {
                            0.0
                        };

                        let telemetry_points = lap.telemetry.len();
                        let has_position_data = lap.telemetry.iter().any(|t| {
                            t.world_position_x.is_some()
                                && t.world_position_y.is_some()
                                && t.world_position_z.is_some()
                        });

                        // Calculate average speed if available
                        let avg_speed = if !lap.telemetry.is_empty() {
                            let speeds: Vec<f32> =
                                lap.telemetry.iter().filter_map(|t| t.speed_mps).collect();
                            if !speeds.is_empty() {
                                Some(speeds.iter().sum::<f32>() / speeds.len() as f32 * 3.6) // Convert to km/h
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        ui.group(|ui| {
                            let _response = ui.selectable_label(
                                is_selected,
                                RichText::new(format!("Lap {}", lap_index + 1))
                                    .color(if is_selected {
                                        Color32::YELLOW
                                    } else {
                                        Color32::WHITE
                                    })
                                    .strong(),
                            );

                            // Show lap metrics
                            ui.horizontal(|ui| {
                                if lap_time > 0.0 {
                                    ui.label(
                                        RichText::new(format!("⏱️ {:.2}s", lap_time))
                                            .color(Color32::LIGHT_BLUE),
                                    );
                                }

                                if let Some(speed) = avg_speed {
                                    ui.label(
                                        RichText::new(format!("🏎️ {:.1} km/h", speed))
                                            .color(Color32::LIGHT_GREEN),
                                    );
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("📊 {} points", telemetry_points))
                                        .color(Color32::GRAY),
                                );

                                if has_position_data {
                                    ui.label(RichText::new("📍 GPS").color(Color32::GREEN));
                                } else {
                                    ui.label(RichText::new("❌ No GPS").color(Color32::RED));
                                }
                            });
                        });

                        ui.add_space(3.0);
                    }
                });
        });
    }

    fn show_svg_preview_panel(&mut self, ui: &mut Ui, session: &Session, selected_lap: &str) {
        egui::CollapsingHeader::new(
            RichText::new("Track Map Preview")
                .color(Color32::WHITE)
                .strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            if let Ok(selected_lap_idx) = selected_lap.parse::<usize>() {
                if let Some(lap) = session.laps.get(selected_lap_idx) {
                    let has_position_data = lap.telemetry.iter().any(|t| {
                        t.world_position_x.is_some()
                            && t.world_position_y.is_some()
                            && t.world_position_z.is_some()
                    });

                    if has_position_data {
                        ui.horizontal(|ui| {
                            if ui.button("🗺️ Generate SVG").clicked() {
                                self.generate_svg_preview(lap);
                            }

                            if self.svg_preview.is_some() && ui.button("💾 Save SVG").clicked() {
                                self.save_svg_to_file();
                            }
                        });

                        // Show SVG preview if available
                        if let Some(ref svg_content) = self.svg_preview {
                            ui.add_space(5.0);
                            ui.group(|ui| {
                                ui.set_max_width(250.0);
                                ui.set_max_height(120.0);

                                egui::ScrollArea::both().show(ui, |ui| {
                                    ui.label(
                                        RichText::new("SVG Preview:").color(Color32::WHITE).small(),
                                    );
                                    ui.add_space(3.0);

                                    // Show a simplified text representation of the SVG
                                    let preview_text = if svg_content.len() > 200 {
                                        format!(
                                            "{}...\n\n[{} characters total]",
                                            &svg_content[..200],
                                            svg_content.len()
                                        )
                                    } else {
                                        svg_content.clone()
                                    };

                                    ui.label(
                                        RichText::new(preview_text)
                                            .color(Color32::LIGHT_GRAY)
                                            .small()
                                            .monospace(),
                                    );
                                });
                            });
                        }
                    } else {
                        ui.label(
                            RichText::new("⚠️ No position data available for SVG generation")
                                .color(Color32::YELLOW),
                        );
                    }
                }
            } else {
                ui.label(RichText::new("Select a lap to preview track map").color(Color32::GRAY));
            }
        });
    }

    fn show_corner_annotation_panel(
        &mut self,
        ui: &mut Ui,
        session: &Session,
        _selected_lap: &str,
    ) {
        egui::CollapsingHeader::new(
            RichText::new("Corner Annotation")
                .color(Color32::WHITE)
                .strong(),
        )
        .default_open(false)
        .show(ui, |ui| {
            // Initialize corner annotation tool if we have SVG and metadata
            if let (Some(svg_content), Some(position_mapping)) =
                (&self.svg_preview, &self.track_position_mapping)
            {
                // Create or update track metadata for corner annotation
                if self.corner_annotation_tool.get_track_metadata().is_none() {
                    let track_id = session.info.track_name.to_lowercase().replace(' ', "_");
                    let metadata = TrackMetadata::new(
                        session.info.track_name.clone(),
                        track_id,
                        svg_content.clone(),
                    );
                    self.corner_annotation_tool
                        .set_track_metadata(metadata, position_mapping.clone());
                }

                // Show interactive SVG with corner annotations
                ui.group(|ui| {
                    ui.set_max_width(350.0);
                    ui.set_max_height(150.0);

                    ui.label(
                        RichText::new("Click on track to add corners:")
                            .color(Color32::WHITE)
                            .small(),
                    );
                    ui.add_space(3.0);

                    self.corner_annotation_tool
                        .show_interactive_svg(ui, svg_content);
                });

                ui.add_space(5.0);

                // Corner management panel
                self.corner_annotation_tool.show_corner_management_panel(ui);
            } else {
                ui.label(
                    RichText::new("Generate SVG first to enable corner annotation")
                        .color(Color32::YELLOW),
                );
            }
        });
    }

    fn show_metadata_tools_panel(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(
            RichText::new("Metadata Tools")
                .color(Color32::WHITE)
                .strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("💾 Save Metadata").clicked() {
                    self.save_track_metadata();
                }

                if ui.button("📂 Load Metadata").clicked() {
                    self.refresh_available_tracks();
                    self.show_load_dialog = true;
                }
            });

            // Show save status message
            if let Some(ref message) = self.save_status_message {
                ui.add_space(3.0);
                let color = if message.starts_with('✓') {
                    Color32::GREEN
                } else {
                    Color32::RED
                };
                ui.colored_label(color, message);
            }

            // Show load status message
            if let Some(ref message) = self.load_status_message {
                ui.add_space(3.0);
                let color = if message.starts_with('✓') {
                    Color32::GREEN
                } else {
                    Color32::RED
                };
                ui.colored_label(color, message);
            }
        });
    }

    fn show_status_panel(&mut self, ui: &mut Ui, session: &Session, selected_lap: &str) {
        egui::CollapsingHeader::new(RichText::new("Status").color(Color32::WHITE).strong())
            .default_open(false)
            .show(ui, |ui| {
                if let Ok(selected_lap_idx) = selected_lap.parse::<usize>() {
                    if let Some(lap) = session.laps.get(selected_lap_idx) {
                        ui.label(
                            RichText::new(format!("✅ Lap {} selected", selected_lap_idx + 1))
                                .color(Color32::GREEN),
                        );

                        ui.label(
                            RichText::new(format!("📊 {} telemetry points", lap.telemetry.len()))
                                .color(Color32::WHITE),
                        );

                        // Show data quality indicators
                        let position_coverage = lap
                            .telemetry
                            .iter()
                            .filter(|t| {
                                t.world_position_x.is_some()
                                    && t.world_position_y.is_some()
                                    && t.world_position_z.is_some()
                            })
                            .count() as f32
                            / lap.telemetry.len() as f32
                            * 100.0;

                        ui.label(
                            RichText::new(format!("📍 Position data: {:.1}%", position_coverage))
                                .color(if position_coverage > 90.0 {
                                    Color32::GREEN
                                } else if position_coverage > 50.0 {
                                    Color32::YELLOW
                                } else {
                                    Color32::RED
                                }),
                        );

                        if self.svg_preview.is_some() {
                            ui.label(RichText::new("🗺️ SVG generated").color(Color32::GREEN));
                        }
                    }
                } else {
                    ui.label(RichText::new("❌ No lap selected").color(Color32::RED));
                }
            });
    }

    /// Generate SVG preview from the selected lap data with comprehensive error handling
    fn generate_svg_preview(&mut self, lap: &Lap) {
        use log::{error, info};

        if lap.telemetry.is_empty() {
            self.save_status_message = Some("⚠ No telemetry data in selected lap".to_string());
            return;
        }

        info!(
            "Generating SVG preview from lap with {} telemetry points",
            lap.telemetry.len()
        );

        // Pre-validate telemetry data
        let position_data_count = lap
            .telemetry
            .iter()
            .filter(|t| {
                t.world_position_x.is_some()
                    && t.world_position_y.is_some()
                    && t.world_position_z.is_some()
            })
            .count();

        if position_data_count == 0 {
            self.save_status_message = Some("⚠ No GPS position data in selected lap".to_string());
            return;
        }

        let coverage = (position_data_count as f32 / lap.telemetry.len() as f32) * 100.0;
        if coverage < 10.0 {
            self.save_status_message = Some(format!(
                "⚠ Insufficient GPS data ({:.1}% coverage)",
                coverage
            ));
            return;
        }

        let generator = TrackMapGenerator::with_config(TrackMapConfig {
            canvas_size: (400, 300), // Smaller size for preview
            stroke_width: 2.0,
            ..Default::default()
        });

        match generator.generate_svg_from_lap(&lap.telemetry) {
            Ok((svg_content, position_mapping)) => {
                info!(
                    "Successfully generated SVG preview ({} characters)",
                    svg_content.len()
                );
                self.svg_preview = Some(svg_content);
                self.track_position_mapping = Some(position_mapping);
                self.save_status_message = Some(format!(
                    "✓ Generated track map ({:.1}% GPS coverage)",
                    coverage
                ));
            }
            Err(e) => {
                error!("Failed to generate SVG preview: {}", e);
                self.svg_preview = None;
                self.track_position_mapping = None;

                // Provide user-friendly error message with recovery suggestions
                let user_message = self.create_svg_generation_error_message(&e);
                self.save_status_message = Some(user_message);
            }
        }
    }

    /// Create user-friendly error message for SVG generation failures
    fn create_svg_generation_error_message(&self, error: &crate::OcypodeError) -> String {
        match error {
            crate::OcypodeError::SvgGenerationError { reason } => {
                if reason.contains("Insufficient position data") {
                    "⚠ Not enough GPS data to create track map. Try a different lap with more position data.".to_string()
                } else if reason.contains("Track geometry too small") {
                    "⚠ Track appears too small. Check if GPS coordinates are in correct units."
                        .to_string()
                } else if reason.contains("Track geometry too elongated") {
                    "⚠ Track shape is too elongated. This may indicate GPS data issues.".to_string()
                } else if reason.contains("No valid points after normalization") {
                    "⚠ GPS coordinates could not be processed. Try a different lap.".to_string()
                } else {
                    format!("⚠ Track map generation failed: {}", reason)
                }
            }
            crate::OcypodeError::TelemetryProducerError { description } => {
                if description.contains("empty lap data") {
                    "⚠ Selected lap has no telemetry data".to_string()
                } else if description.contains("Insufficient position data points") {
                    "⚠ Not enough GPS points to create track map (need at least 3)".to_string()
                } else {
                    format!("⚠ Telemetry data error: {}", description)
                }
            }
            _ => "⚠ Failed to generate track map. Try selecting a different lap.".to_string(),
        }
    }

    /// Save the generated SVG to a file
    fn save_svg_to_file(&self) {
        if let Some(ref _svg_content) = self.svg_preview {
            // For now, just log that we would save the file
            // In a full implementation, this would open a file dialog
            log::info!("Would save SVG file with {} characters", _svg_content.len());

            // TODO: Implement actual file saving with file dialog
            // This would typically use rfd::FileDialog or similar
        }
    }

    /// Save track metadata including corner annotations
    fn save_track_metadata(&mut self) {
        if let Some(metadata) = self.corner_annotation_tool.get_track_metadata() {
            // Clone the metadata with all corner annotations
            self.generated_metadata = Some(metadata.clone());

            // Check if metadata already exists and show confirmation if needed
            if let Some(ref mut storage) = self.metadata_storage {
                match storage.metadata_exists(&metadata.track_id) {
                    Ok(true) => {
                        // Metadata exists, show confirmation dialog
                        self.show_save_confirmation = true;
                    }
                    Ok(false) => {
                        // Metadata doesn't exist, proceed with save
                        self.perform_metadata_save();
                    }
                    Err(e) => {
                        self.save_status_message =
                            Some(format!("Error checking existing metadata: {}", e));
                        log::error!("Error checking existing metadata: {}", e);
                    }
                }
            } else {
                self.save_status_message = Some("Storage not initialized".to_string());
            }
        } else if let (Some(_svg_content), Some(_position_mapping)) =
            (&self.svg_preview, &self.track_position_mapping)
        {
            // Fallback: create basic metadata without corners
            self.save_status_message = Some("No metadata to save. Generate SVG first.".to_string());
        } else {
            self.save_status_message = Some("No metadata to save. Generate SVG first.".to_string());
        }
    }

    /// Perform the actual metadata save operation
    fn perform_metadata_save(&mut self) {
        if let (Some(metadata), Some(storage)) =
            (&self.generated_metadata, &mut self.metadata_storage)
        {
            match storage.save_metadata(metadata) {
                Ok(()) => {
                    self.save_status_message = Some(format!(
                        "✓ Successfully saved track map for {}",
                        metadata.track_name
                    ));
                    self.load_status_message = None;
                }
                Err(e) => {
                    log::error!("Failed to save track metadata: {}", e);
                    self.save_status_message = Some(format!("⚠ Failed to save track map: {}", e));
                }
            }
        } else {
            self.save_status_message =
                Some("⚠ No track metadata to save. Generate SVG first.".to_string());
        }

        // Clear confirmation dialog
        self.show_save_confirmation = false;
    }

    /// Refresh the list of available tracks
    fn refresh_available_tracks(&mut self) {
        if let Some(ref storage) = self.metadata_storage {
            match storage.list_available_tracks() {
                Ok(tracks) => {
                    self.available_tracks = tracks;
                }
                Err(e) => {
                    log::error!("Failed to list available tracks: {}", e);
                    self.available_tracks.clear();
                }
            }
        }
    }
}
