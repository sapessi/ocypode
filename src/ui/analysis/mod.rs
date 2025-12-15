use std::{path::PathBuf, sync::Arc};

use egui::{
    Align, Color32, Direction, Frame, Label, Layout, Margin, RichText, Ui, Vec2b, Visuals, containers::CentralPanel, style::Widgets
};
use egui_dropdown::DropDownBox;
use egui_plot::{Legend, Line, PlotPoints, Points};
use itertools::Itertools;
use log;

use crate::{
    telemetry::TelemetryAnnotation,
    track_metadata::{TrackMetadata, TrackMetadataStorage},
    ui::live::{PALETTE_BLACK, PALETTE_BROWN, PALETTE_MAROON, PALETTE_ORANGE},
};

use super::{Alert, DefaultAlert, ScrubSlipAlert, stroke_shade};

mod data_types;
mod developer_ui;
mod telemetry_loader;

use data_types::{Lap, Session, TelemetryFile, UiState};
use developer_ui::DeveloperUiState;

pub(crate) struct TelemetryAnalysisApp<'file> {
    source_file: Option<&'file PathBuf>,
    ui_state: UiState,
    data: Option<TelemetryFile>,
    selected_session: String,
    selected_lap: String,
    comparison_lap: String,
    selected_annotation_content: String,
    selected_x: Option<usize>,
    is_developer_mode: bool,
    // Developer mode UI state
    developer_ui: DeveloperUiState,
    // Load interface track metadata integration
    loaded_track_metadata: Option<TrackMetadata>,
    track_metadata_lookup_attempted: bool,

    files_loaded: Vec<TelemetryFileState>,
}

pub(crate) struct TelemetryFileState {
    source_file: PathBuf,
    data: TelemetryFile,
    selected_session: String,
    selected_lap: String,
    comparison_lap: String,
}

impl<'file> TelemetryAnalysisApp<'file> {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This gives us image support:
        egui_extras::install_image_loaders(&cc.egui_ctx);


        Self {
            source_file: None,
            ui_state: UiState::Loading,
            data: None,
            selected_session: "".to_string(),
            selected_lap: "".to_string(),
            comparison_lap: "".to_string(),
            selected_annotation_content: "".to_string(),
            selected_x: None,
            is_developer_mode: false,
            developer_ui: DeveloperUiState::default(),
            loaded_track_metadata: None,
            track_metadata_lookup_attempted: false,

            files_loaded: Vec::new(),
        }
    }

    fn show_selectors(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            let sessions = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .map(|i| i.info.track_name.as_str());

            let previous_session = self.selected_session.clone();

            ui.label(RichText::new("Session: ").color(Color32::WHITE));
            ui.add(
                DropDownBox::from_iter(
                    sessions,
                    "session_dropbox",
                    &mut self.selected_session,
                    |ui, text| ui.selectable_label(false, text),
                )
                .filter_by_input(false),
            );

            // Reset track metadata lookup if session changed
            if previous_session != self.selected_session {
                self.track_metadata_lookup_attempted = false;
                self.loaded_track_metadata = None;
            }

            if let Some(selected_session) = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .find(|p| p.info.track_name == self.selected_session)
            {
                ui.separator();
                ui.label(RichText::new("Lap: ").color(Color32::WHITE));
                let laps_iter = (0..selected_session.laps.len())
                    .map(|l| l.to_string())
                    .collect_vec();
                ui.add(
                    DropDownBox::from_iter(
                        laps_iter,
                        "lap_dropbox",
                        &mut self.selected_lap,
                        |ui, text| ui.selectable_label(false, text),
                    )
                    .filter_by_input(false),
                );
            }

            if let Some(selected_session) = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .find(|p| p.info.track_name == self.selected_session)
            {
                ui.separator();
                ui.label(RichText::new("Comparison lap: ").color(Color32::WHITE));
                let laps_iter = (0..selected_session.laps.len())
                    .map(|l| l.to_string())
                    .collect_vec();
                ui.add(
                    DropDownBox::from_iter(
                        laps_iter,
                        "comparison_lap_dropbox",
                        &mut self.comparison_lap,
                        |ui, text| ui.selectable_label(false, text),
                    )
                    .filter_by_input(false),
                );
            }
        });
    }

    fn show_telemetry_chart(&mut self, selected_lap: usize, session: &Session, ui: &mut Ui) {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            let plot = egui_plot::Plot::new("measurements");
            //println!("Selected lap = {}", selected_lap);
            if let Some(lap) = session.laps.get(selected_lap) {
                let mut throttle_vec = Vec::<[f64; 2]>::new();
                let mut brake_vec = Vec::<[f64; 2]>::new();
                let mut steering_vec = Vec::<[f64; 2]>::new();
                let mut annotations_vec = Vec::<[f64; 2]>::new();

                lap.telemetry.iter().enumerate().all(|p| {
                    let throttle = p.1.throttle.unwrap_or(0.0);
                    let brake = p.1.brake.unwrap_or(0.0);
                    let steering_pct = p.1.steering_pct.unwrap_or(0.0);
                    throttle_vec.push([p.0 as f64, throttle as f64 * 100.]);
                    brake_vec.push([p.0 as f64, brake as f64 * 100.]);
                    steering_vec.push([p.0 as f64, 50. + 50. * steering_pct as f64]);
                    if !p.1.annotations.is_empty() {
                        annotations_vec.push([p.0 as f64, 101.]);
                    }
                    true
                });

                let throttle_points = PlotPoints::new(throttle_vec);
                let brake_points = PlotPoints::new(brake_vec);
                let steering_points = PlotPoints::new(steering_vec);
                let annotation_points = PlotPoints::new(annotations_vec);

                let plot_response = plot
                    .show_background(false)
                    .legend(Legend::default())
                    .include_y(0.)
                    .include_y(150.)
                    .include_x(0.)
                    .include_x(250.) // TODO: make this dynamic based on window size
                    .auto_bounds(Vec2b::new(false, false))
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("Throttle", throttle_points)
                                .color(Color32::GREEN)
                                .fill(0.),
                        );
                        plot_ui.line(
                            Line::new("Brake", brake_points)
                                .gradient_color(
                                    Arc::new(|point| {
                                        stroke_shade(
                                            PALETTE_ORANGE,
                                            Color32::RED,
                                            (point.y / 100.) as f32,
                                        )
                                    }),
                                    true,
                                )
                                .color(Color32::RED)
                                .fill(0.),
                        );
                        plot_ui.line(
                            Line::new("Steering", steering_points).color(Color32::LIGHT_GRAY),
                        );
                        plot_ui.points(
                            Points::new("Annotation", annotation_points)
                                .color(Color32::BLUE)
                                .radius(10.),
                        );

                        if !self.comparison_lap.is_empty()
                            && let Some(comparison_lap) = session
                                .laps
                                .get(self.comparison_lap.parse::<usize>().unwrap())
                        {
                            let comparison_throttle_points = PlotPoints::new(
                                comparison_lap
                                    .telemetry
                                    .iter()
                                    .enumerate()
                                    .map(|t| {
                                        let throttle = t.1.throttle.unwrap_or(0.0);
                                        [t.0 as f64, throttle as f64 * 100.]
                                    })
                                    .collect(),
                            );
                            let comparison_brake_points = PlotPoints::new(
                                comparison_lap
                                    .telemetry
                                    .iter()
                                    .enumerate()
                                    .map(|t| {
                                        let brake = t.1.brake.unwrap_or(0.0);
                                        [t.0 as f64, brake as f64 * 100.]
                                    })
                                    .collect(),
                            );
                            let comparison_steering_points = PlotPoints::new(
                                comparison_lap
                                    .telemetry
                                    .iter()
                                    .enumerate()
                                    .map(|t| {
                                        let steering_pct = t.1.steering_pct.unwrap_or(0.0);
                                        [t.0 as f64, 50. + 50. * steering_pct as f64]
                                    })
                                    .collect(),
                            );

                            plot_ui.line(
                                Line::new("Comparison Throttle", comparison_throttle_points)
                                    .color(Color32::DARK_GREEN),
                            );
                            plot_ui.line(
                                Line::new("Comparison Brake", comparison_brake_points)
                                    .color(Color32::DARK_RED),
                            );
                            plot_ui.line(
                                Line::new("Comparison Steering", comparison_steering_points)
                                    .color(Color32::DARK_GRAY.gamma_multiply(0.3)),
                            );
                        }
                    });
                if plot_response.response.clicked()
                    && let Some(mouse_pos) = plot_response.response.interact_pointer_pos()
                {
                    self.selected_annotation_content = "".to_string();
                    self.selected_x = Some(
                        plot_response
                            .transform
                            .value_from_position(mouse_pos)
                            .x
                            .floor() as usize,
                    );
                }
            }
        });
    }

    /// Clear status messages after they've been displayed for a while
    fn clear_old_status_messages(&mut self) {
        // This is a simple implementation - in a real app you might want to use timestamps
        // For now, we'll just clear messages when dialogs are closed
        if !self.developer_ui.show_save_confirmation && !self.developer_ui.show_load_dialog {
            // Clear messages after successful operations or when no dialogs are open
            if let Some(ref msg) = self.developer_ui.save_status_message
                && msg.starts_with('✓')
            {
                // Keep success messages a bit longer, but clear them eventually
                // This is a simplified approach - you could use a timer here
            }
        }
    }

    /// Attempt to load track metadata for the current session with comprehensive error handling
    /// This implements Requirements 5.1 and 5.5 - check for available track metadata and graceful degradation
    fn attempt_track_metadata_lookup(&mut self, track_name: &str) {
        use log::{debug, error, info, warn};

        if self.track_metadata_lookup_attempted {
            return;
        }

        self.track_metadata_lookup_attempted = true;
        debug!("Attempting track metadata lookup for: {}", track_name);

        // Validate track name before lookup
        if track_name.is_empty() {
            warn!("Cannot lookup metadata for empty track name");
            self.loaded_track_metadata = None;
            self.developer_ui.load_status_message = Some("⚠ Track name is empty".to_string());
            return;
        }

        if track_name == "Unknown" {
            debug!("Skipping metadata lookup for unknown track");
            self.loaded_track_metadata = None;
            self.developer_ui.load_status_message =
                Some("ℹ Track identification needed for metadata".to_string());
            return;
        }

        if let Some(ref storage) = self.developer_ui.metadata_storage {
            match storage.load_metadata(track_name) {
                Ok(Some(metadata)) => {
                    info!("Successfully loaded track metadata for: {}", track_name);
                    self.loaded_track_metadata = Some(metadata);
                    self.developer_ui.load_status_message =
                        Some(format!("✓ Loaded track map for {}", track_name));
                }
                Ok(None) => {
                    debug!("No track metadata found for: {}", track_name);
                    self.loaded_track_metadata = None;
                    self.developer_ui.load_status_message =
                        Some(format!("ℹ No track map available for {}", track_name));
                }
                Err(e) => {
                    error!("Failed to load track metadata for {}: {}", track_name, e);
                    self.loaded_track_metadata = None;

                    // Provide user-friendly error messages with recovery suggestions
                    let user_message = self.create_metadata_error_message(&e, track_name);
                    self.developer_ui.load_status_message = Some(user_message);
                }
            }
        } else {
            warn!("Metadata storage not available for track metadata lookup");
            self.loaded_track_metadata = None;
            self.developer_ui.load_status_message =
                Some("⚠ Track metadata system not available".to_string());
        }
    }

    /// Create user-friendly error message for metadata loading failures
    fn create_metadata_error_message(
        &self,
        error: &crate::OcypodeError,
        track_name: &str,
    ) -> String {
        use crate::OcypodeError;

        match error {
            OcypodeError::TrackMetadataValidationError { reason } => {
                format!(
                    "⚠ Track map data is corrupted for {}: {}",
                    track_name, reason
                )
            }
            OcypodeError::TrackMetadataStorageError { reason } => {
                if reason.contains("Failed to read file") {
                    format!("⚠ Cannot read track map file for {}", track_name)
                } else if reason.contains("Failed to parse JSON") {
                    format!("⚠ Track map file is corrupted for {}", track_name)
                } else {
                    format!("⚠ Track map storage error for {}: {}", track_name, reason)
                }
            }
            OcypodeError::FileOperationError { operation, reason } => match operation.as_str() {
                "read_metadata_file" => {
                    format!("⚠ Cannot access track map file for {}", track_name)
                }
                "load_backup" => format!(
                    "⚠ Track map backup files are also corrupted for {}",
                    track_name
                ),
                _ => format!(
                    "⚠ File system error loading track map for {}: {}",
                    track_name, reason
                ),
            },
            OcypodeError::InvalidUserInput { field, reason } => {
                format!("⚠ Invalid track name '{}': {}", field, reason)
            }
            _ => {
                format!("⚠ Failed to load track map for {}", track_name)
            }
        }
    }

    /// Show track map panel alongside telemetry charts
    /// This implements Requirement 5.2 - display SVG track map alongside telemetry information
    fn show_track_map_panel(&mut self, ui: &mut Ui, session: &Session) {
        ui.group(|ui| {
            ui.set_max_width(300.0);
            ui.set_max_height(250.0);

            ui.label(RichText::new("Track Map").color(Color32::WHITE).strong());
            ui.separator();

            if let Some(ref metadata) = self.loaded_track_metadata {
                // Display SVG track map
                ui.label(RichText::new(&metadata.track_name).color(Color32::WHITE));
                ui.add_space(5.0);

                // Show SVG preview (simplified text representation for now)
                egui::ScrollArea::both().max_height(150.0).show(ui, |ui| {
                    let svg_preview = if metadata.svg_map.len() > 150 {
                        format!(
                            "{}...\n\n[SVG Map Available - {} characters]",
                            &metadata.svg_map[..150],
                            metadata.svg_map.len()
                        )
                    } else {
                        metadata.svg_map.clone()
                    };

                    ui.label(
                        RichText::new(svg_preview)
                            .color(Color32::LIGHT_GRAY)
                            .small()
                            .monospace(),
                    );
                });

                ui.add_space(5.0);

                // Show corner information if available
                if !metadata.corners.is_empty() {
                    ui.label(
                        RichText::new(format!("Corners: {}", metadata.corners.len()))
                            .color(Color32::LIGHT_BLUE),
                    );

                    // Show corner details in a compact format
                    egui::ScrollArea::vertical()
                        .max_height(60.0)
                        .show(ui, |ui| {
                            for corner in &metadata.corners {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("C{}", corner.corner_number))
                                            .color(Color32::YELLOW)
                                            .small(),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{:.1}%-{:.1}%",
                                            corner.track_percentage_start * 100.0,
                                            corner.track_percentage_end * 100.0
                                        ))
                                        .color(Color32::GRAY)
                                        .small(),
                                    );
                                });
                            }
                        });
                } else {
                    ui.label(
                        RichText::new("No corner annotations")
                            .color(Color32::GRAY)
                            .small(),
                    );
                }
            } else {
                // Graceful degradation when no metadata is available (Requirement 5.5)
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("No track map available").color(Color32::GRAY));
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(format!("Track: {}", session.info.track_name))
                            .color(Color32::WHITE)
                            .small(),
                    );

                    if self.is_developer_mode {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("💡 Use developer mode to create track metadata")
                                .color(Color32::YELLOW)
                                .small(),
                        );
                    }
                });
            }
        });
    }
}

impl eframe::App for TelemetryAnalysisApp<'_> {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // Give the area behind the floating windows a different color, because it looks better:
        let color = egui::lerp(
            egui::Rgba::from(visuals.panel_fill)..=egui::Rgba::from(visuals.extreme_bg_color),
            0.5,
        );
        let color = egui::Color32::from(color);
        color.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("wrap_app_top_bar")
            .frame(egui::Frame::new().inner_margin(4))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.visuals_mut().button_frame = false;
                    if ui.button("📂 Load Telemetry").clicked() && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        let new_telemetry_file = TelemetryFileState {
                            source_file: path.clone(),
                            data: telemetry_loader::load_telemetry_jsonl(&path).expect("Could not load telemetry file"),    
                            selected_session: "".to_string(),
                            selected_lap: "".to_string(),
                            comparison_lap: "".to_string(),
                        };
                        self.files_loaded.push(new_telemetry_file);
                    }
                });
            });

        egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
            for tf in self.files_loaded.iter() {
                // Render UI for each loaded telemetry file
                egui::Window::new(tf.source_file.to_str().unwrap().to_owned())
                    .default_width(320.0)
                    .default_height(480.0)
                    .resizable([true, false])
                    .scroll(false)
                    .show(ctx, |ui| {
                        ui.label(format!("Telemetry file: {:?}", tf.source_file));
                    });
            }
        });
    }
    /*
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        let cur_ui_state = self.ui_state.clone();
        match cur_ui_state {
            UiState::Loading => {
                if self.data.is_none() {
                    let telemetry_load_result =
                        telemetry_loader::load_telemetry_jsonl(self.source_file.unwrap());
                    if telemetry_load_result.is_err() {
                        self.ui_state = UiState::Error {
                            message: format!(
                                "Could not load telemetry: {}",
                                telemetry_load_result.err().unwrap()
                            ),
                        };
                        return;
                    }
                    self.data = Some(telemetry_load_result.unwrap());
                    self.ui_state = UiState::Display {
                        session: self
                            .data
                            .as_ref()
                            .unwrap()
                            .sessions
                            .first()
                            .unwrap()
                            .clone(),
                    }
                }
            }
            UiState::Display { session } => {
                // Attempt to load track metadata for the current session (Requirements 5.1, 5.5)
                self.attempt_track_metadata_lookup(&session.info.track_name);

                // Show developer mode indicator in the top panel if enabled
                egui::TopBottomPanel::top("SessionSelector")
                    .frame(
                        Frame::default()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::same(5)),
                    )
                    .resizable(false)
                    .min_height(40.0)
                    .max_height(80.0)
                    .show(ctx, |local_ui| {
                        if self.is_developer_mode {
                            local_ui.with_layout(
                                Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("🔧 Developer Mode - Track Metadata Manager")
                                            .color(Color32::YELLOW)
                                            .strong(),
                                    );
                                    ui.separator();
                                    ui.label(
                                        RichText::new(format!(
                                            "Track: {}",
                                            session.info.track_name
                                        ))
                                        .color(Color32::WHITE),
                                    );
                                },
                            );
                            local_ui.separator();
                        } else {
                            // Show track metadata status in regular mode
                            local_ui.with_layout(
                                Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Track: {}",
                                            session.info.track_name
                                        ))
                                        .color(Color32::WHITE),
                                    );

                                    if self.loaded_track_metadata.is_some() {
                                        ui.separator();
                                        ui.label(
                                            RichText::new("🗺️ Track map available")
                                                .color(Color32::GREEN)
                                                .small(),
                                        );
                                    }
                                },
                            );
                        }
                        self.show_selectors(local_ui);
                    });

                // Show developer mode panel on the left if enabled
                if self.is_developer_mode {
                    egui::SidePanel::left("DeveloperModeControls")
                        .frame(
                            Frame::default()
                                .fill(Color32::TRANSPARENT)
                                .inner_margin(Margin::same(5)),
                        )
                        .resizable(true)
                        .min_width(250.0)
                        .max_width(400.0)
                        .show(ctx, |ui| {
                            // Constrain the developer panel to not take up the full screen height
                            egui::ScrollArea::vertical()
                                .max_height(ctx.available_rect().height() * 0.7) // Limit to 70% of screen height
                                .show(ui, |ui| {
                                    self.developer_ui.show_developer_mode_panel(
                                        ui,
                                        &session,
                                        &self.selected_lap,
                                    );
                                });
                        });
                } else if self.loaded_track_metadata.is_some() {
                    // Show track map panel in regular mode when metadata is available (Requirement 5.2)
                    egui::SidePanel::left("TrackMapPanel")
                        .frame(
                            Frame::default()
                                .fill(Color32::TRANSPARENT)
                                .inner_margin(Margin::same(5)),
                        )
                        .resizable(true)
                        .min_width(200.0)
                        .max_width(350.0)
                        .show(ctx, |ui| {
                            self.show_track_map_panel(ui, &session);
                        });
                }

                egui::SidePanel::right("AnnotationDetail")
                    .frame(
                        Frame::default()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::same(5)),
                    )
                    .resizable(false)
                    .min_width(ctx.available_rect().width() * 0.3)
                    .max_width(ctx.available_rect().height() / 7.)
                    .show(ctx, |local_ui| {
                        if let Ok(selected_lap) = self.selected_lap.parse::<usize>() {
                            if let (Some(x_point), Some(lap)) = (self.selected_x, session.laps.get(selected_lap)) &&
                                 let Some(telemetry) = lap.telemetry.get(x_point) {
                                        let mut abs_alert = DefaultAlert::abs().button();
                                        let mut shift_alert = DefaultAlert::shift().button();
                                        let mut traction_alert = DefaultAlert::traction().button();
                                        let mut trailbrake_steering_alert = DefaultAlert::trailbrake_steering().button();
                                        let mut slip_alert = ScrubSlipAlert::default().button();

                                        let _ = abs_alert.update_state(telemetry);
                                        let _ = shift_alert.update_state(telemetry);
                                        let _ = traction_alert.update_state(telemetry);
                                        let _ = trailbrake_steering_alert.update_state(telemetry);
                                        let _ = slip_alert.update_state(telemetry);

                                        local_ui.with_layout(Layout::top_down(Align::Center), |ui| {
                                            if abs_alert.show(ui, Align::Center).clicked() {
                                                let brake = telemetry.brake.unwrap_or(0.0);
                                                self.selected_annotation_content = format!("brake force: {:.2}", brake);
                                            };
                                            ui.separator();
                                            if shift_alert.show(ui, Align::Center).clicked() && let Some(TelemetryAnnotation::ShortShifting { gear_change_rpm, optimal_rpm, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::ShortShifting { .. })) {
                                                        let cur_gear = telemetry.gear.unwrap_or(0);
                                                        self.selected_annotation_content = format!(
                                                            "From gear: {}\nTo gear: {}\nIdeal RPM: {}\nActual RPM: {}",
                                                            cur_gear - 1,
                                                            cur_gear,
                                                            optimal_rpm,
                                                            gear_change_rpm
                                                        )
                                            }
                                            ui.separator();
                                            if traction_alert.show(ui, Align::Center).clicked() && let Some(TelemetryAnnotation::Wheelspin { avg_rpm_increase_per_gear, cur_gear, cur_rpm_increase, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::Wheelspin { .. })) {
                                                        self.selected_annotation_content = format!(
                                                            "Gear: {}\nRPM increase: {:.1}\np90 RPM increase: {:.1}\nRPM increase per gear:\n{}",
                                                            cur_gear,
                                                            cur_rpm_increase,
                                                            avg_rpm_increase_per_gear.get(cur_gear).unwrap(),
                                                            serde_json::to_string_pretty(avg_rpm_increase_per_gear).unwrap()
                                                        );
                                            }
                                            ui.separator();
                                            if trailbrake_steering_alert.show(ui, Align::Center).clicked() && let Some(TelemetryAnnotation::TrailbrakeSteering { cur_trailbrake_steering, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::TrailbrakeSteering { .. })) {
                                                        let steering = telemetry.steering_angle_rad.unwrap_or(0.0);
                                                        self.selected_annotation_content = format!(
                                                            "Steering: {:.2}%\nSteering angle (rad): {}",
                                                            cur_trailbrake_steering,
                                                            steering
                                                        );
                                            }
                                            ui.separator();
                                            if slip_alert.show(ui, Align::Center).clicked() {
                                                if let Some(TelemetryAnnotation::Scrub { avg_yaw_rate_change, cur_yaw_rate_change, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::Scrub { .. })) {
                                                        let steering = telemetry.steering_angle_rad.unwrap_or(0.0);
                                                        let speed = telemetry.speed_mps.unwrap_or(0.0);
                                                        self.selected_annotation_content = format!(
                                                            "Yaw change: {:.2}\nAvg yaw change: {:.2}\nSteering (rad): {:.2}\nSpeed: {:.2}",
                                                            cur_yaw_rate_change,
                                                            avg_yaw_rate_change,
                                                            steering,
                                                            speed
                                                        );
                                                }
                                                if let Some(TelemetryAnnotation::Slip { prev_speed, cur_speed, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::Slip { .. })) {
                                                        let throttle = telemetry.throttle.unwrap_or(0.0);
                                                        let steering = telemetry.steering_angle_rad.unwrap_or(0.0);
                                                        self.selected_annotation_content = format!(
                                                            "Speed: {:.2}\nPrev speed: {:.2}\nThrottle %: {:.2}%\nSteering (rad): {:.2}%",
                                                            cur_speed,
                                                            prev_speed,
                                                            throttle,
                                                            steering
                                                        );
                                                }
                                            }
                                        });

                                        local_ui.add(
                                            Label::new(RichText::new(self.selected_annotation_content.clone()).color(Color32::WHITE))
                                        );
                                }
                            } else {
                                local_ui.with_layout(
                                    Layout::centered_and_justified(Direction::TopDown),
                                    |ui| {
                                        ui.label(
                                            RichText::new("No telemetry point selected")
                                                .color(Color32::WHITE)
                                                .strong(),
                                        );
                                    },
                                );
                            }
                    });
                egui::CentralPanel::default()
                    .frame(
                        Frame::default()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::same(5)),
                    )
                    .show(ctx, |local_ui| {
                        if let Ok(selected_lap) = self.selected_lap.parse::<usize>() {
                            self.show_telemetry_chart(selected_lap, &session, local_ui);
                        }
                    });

                // Show corner annotation dialog if needed (developer mode only)
                if self.is_developer_mode {
                    // Create a temporary UI for the dialog
                    egui::Area::new(egui::Id::new("corner_dialog")).show(ctx, |ui| {
                        self.developer_ui
                            .corner_annotation_tool
                            .show_corner_input_dialog(ui);
                    });

                    // Show save confirmation dialog
                    if self.developer_ui.show_save_confirmation {
                        // TODO: Move dialog to developer_ui module
                    }

                    // Show load dialog
                    if self.developer_ui.show_load_dialog {
                        // TODO: Move dialog to developer_ui module
                    }
                }
            }

            UiState::Error { message } => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading(RichText::new(message).color(Color32::RED).strong());
                });
            }
        }
        // Clear status messages after some time (in developer mode)
        if self.is_developer_mode {
            self.clear_old_status_messages();
        }

        ctx.request_repaint();
    }
     */
}
