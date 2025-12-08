use egui::{Align, Color32, CornerRadius, Frame, Id, Layout, RichText, Sense, ViewportCommand};

use super::{DEFAULT_WINDOW_CORNER_RADIUS, LiveTelemetryApp};

impl LiveTelemetryApp {
    /// Display the Setup Window viewport.
    ///
    /// This window shows detected handling issues (findings) and provides
    /// setup recommendations for confirmed findings. It follows the same
    /// visual styling as the alerts window.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 2.1, 5.1, 10.1, 10.2, 10.3, 10.4:
    /// - Displays findings in a separate viewport
    /// - Uses same visual styling as alerts window
    /// - Supports draggable repositioning
    /// - Maintains consistent UI design
    pub(crate) fn setup_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel with draggable area for repositioning
        egui::TopBottomPanel::top("setup_controls")
            .min_height(30.)
            .frame(Frame::new().corner_radius(CornerRadius {
                nw: DEFAULT_WINDOW_CORNER_RADIUS,
                ne: DEFAULT_WINDOW_CORNER_RADIUS,
                ..Default::default()
            }))
            .show(ctx, |ui| {
                // Make the entire top panel draggable
                let drag_sense =
                    ui.interact(ui.max_rect(), Id::new("setup-window-drag"), Sense::drag());
                if drag_sense.dragged() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                // Save position when drag stops
                if drag_sense.drag_stopped() {
                    if let Some(outer_rect) = ui.input(|is| is.viewport().outer_rect) {
                        self.app_config.setup_window_position = outer_rect.min.into();
                        // Save config immediately to persist position
                        if let Err(e) = self.app_config.save() {
                            log::error!("Failed to save config after window drag: {}", e);
                        }
                    };
                }

                // Window title and controls
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.heading(RichText::new("Setup Assistant").color(Color32::WHITE));

                    // Add spacing to push button to the right
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Clear findings button
                        if ui.button("Clear Findings").clicked() {
                            self.setup_assistant.clear_session();

                            // Save cleared state to config
                            self.app_config.setup_assistant_findings =
                                self.setup_assistant.get_findings_for_persistence().clone();
                            self.app_config.setup_assistant_confirmed_findings = self
                                .setup_assistant
                                .get_confirmed_findings_for_persistence()
                                .clone();

                            // Save config immediately to persist cleared state
                            if let Err(e) = self.app_config.save() {
                                log::error!("Failed to save config after clearing findings: {}", e);
                            }
                        }
                    });
                });
            });

        // Central panel with findings and recommendations
        egui::CentralPanel::default()
            .frame(Frame::new().corner_radius(CornerRadius {
                sw: DEFAULT_WINDOW_CORNER_RADIUS,
                se: DEFAULT_WINDOW_CORNER_RADIUS,
                ..Default::default()
            }))
            .show(ctx, |ui| {
                self.show_findings_list(ui);
            });
    }

    /// Display the list of detected findings.
    ///
    /// Shows each finding with its type, occurrence count, and corner phase.
    /// Findings are clickable to toggle confirmation. Confirmed findings are
    /// visually distinguished from unconfirmed ones.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 2.2, 2.3, 2.4, 3.1, 3.2:
    /// - Displays each finding with type, count, and corner phase
    /// - Makes findings clickable for confirmation
    /// - Visually distinguishes confirmed vs unconfirmed findings
    /// - Shows "No issues detected" when findings list is empty
    /// - Updates findings list in real-time as new telemetry arrives
    /// - Maintains scroll position during updates
    ///
    /// # UI Polish
    ///
    /// - Consistent spacing between elements
    /// - Clear visual hierarchy with headings
    /// - Responsive layout that adapts to content
    /// - Smooth scrolling with preserved position
    fn show_findings_list(&mut self, ui: &mut egui::Ui) {
        // Clone findings to avoid borrow conflicts with the scroll area closure
        // This is efficient as findings are typically small (< 20 items)
        let findings: Vec<_> = self
            .setup_assistant
            .get_findings()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Show "No issues detected" message when findings list is empty
        if findings.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(egui::RichText::new("No issues detected").size(16.0));
                ui.add_space(15.0);
                ui.label(
                    egui::RichText::new("Drive a few laps to collect data")
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(20.0);
            });
            return;
        }

        // Use a scroll area with a stable ID to maintain scroll position during updates
        // This ensures the UI remains responsive and scroll position is preserved
        // as new telemetry arrives and occurrence counts are updated
        egui::ScrollArea::vertical()
            .id_salt("setup_findings_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Display findings list with improved spacing
                ui.add_space(5.0);
                ui.heading("Detected Issues");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Click an issue to confirm and see recommendations")
                        .size(12.0)
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(12.0);

                // Sort findings by occurrence count (most frequent first)
                // This provides a stable ordering that helps maintain context during updates
                let mut findings_vec = findings;
                findings_vec.sort_by(|a, b| b.1.occurrence_count.cmp(&a.1.occurrence_count));

                // Track which finding was clicked (if any)
                let mut clicked_finding: Option<crate::setup_assistant::FindingType> = None;

                for (finding_type, finding) in findings_vec {
                    let is_confirmed = self.setup_assistant.is_confirmed(&finding_type);

                    // Create a selectable label for each finding
                    // Occurrence count updates in real-time as new telemetry is processed
                    let finding_text = RichText::new(format!(
                        "{} ({}) - {}",
                        finding_type, finding.occurrence_count, finding.corner_phase
                    ))
                    .color(Color32::WHITE);

                    // Use different styling for confirmed vs unconfirmed findings
                    let response = if is_confirmed {
                        // Confirmed findings: use a filled button style
                        ui.selectable_label(true, finding_text)
                    } else {
                        // Unconfirmed findings: use a regular selectable label
                        ui.selectable_label(false, finding_text)
                    };

                    // Track click for later processing
                    if response.clicked() {
                        clicked_finding = Some(finding_type.clone());
                    }

                    // Add consistent spacing between findings
                    ui.add_space(6.0);
                }

                // Toggle confirmation after the loop to avoid borrow conflicts
                if let Some(finding_type) = clicked_finding {
                    self.setup_assistant.toggle_confirmation(finding_type);

                    // Save setup assistant state to config after confirmation change
                    self.app_config.setup_assistant_findings =
                        self.setup_assistant.get_findings_for_persistence().clone();
                    self.app_config.setup_assistant_confirmed_findings = self
                        .setup_assistant
                        .get_confirmed_findings_for_persistence()
                        .clone();

                    // Save config immediately to persist confirmation state
                    if let Err(e) = self.app_config.save() {
                        log::error!("Failed to save config after confirmation toggle: {}", e);
                    }
                }

                // Show recommendations section for all confirmed findings
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(5.0);
                self.show_recommendations(ui);
            });
    }

    /// Display setup recommendations for confirmed findings.
    ///
    /// Shows recommendations grouped by setup category, with parameter name,
    /// adjustment direction, and description for each recommendation.
    /// Supports displaying recommendations for multiple confirmed findings.
    /// Updates in real-time as findings are confirmed or unconfirmed.
    ///
    /// # Requirements
    ///
    /// Implements Requirements 2.4, 3.3, 3.5, 4.2, 4.3, 4.4:
    /// - Displays recommendations for confirmed findings
    /// - Groups recommendations by setup category
    /// - Shows parameter name, adjustment direction, and description
    /// - Supports displaying multiple recommendation sets
    /// - Updates in real-time as confirmation state changes
    /// - Prioritizes recommendations by impact
    /// - Highlights conflicting recommendations
    fn show_recommendations(&self, ui: &mut egui::Ui) {
        // Get processed recommendations with priority and conflict detection
        let processed_recommendations = self.setup_assistant.get_processed_recommendations();

        // If no confirmed findings, show a message
        if processed_recommendations.is_empty() {
            ui.add_space(15.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Click on an issue above to see recommendations")
                        .color(egui::Color32::GRAY),
                );
            });
            return;
        }

        // Display recommendations section header with improved spacing
        ui.add_space(5.0);
        ui.heading("Setup Recommendations");
        ui.add_space(8.0);

        // Show priority info
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Sorted by priority • ")
                    .size(12.0)
                    .color(egui::Color32::GRAY),
            );
            ui.label(
                egui::RichText::new("⚠️ = Conflicting recommendations")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(255, 200, 100)),
            );
        });
        ui.add_space(12.0);

        // Display recommendations in priority order (already sorted by process_recommendations)
        for proc_rec in &processed_recommendations {
            let rec = &proc_rec.recommendation;

            // Priority badge, category, parameter, and adjustment on one line
            ui.horizontal(|ui| {
                // Priority badge with color coding
                let priority_color = match rec.priority {
                    5 => egui::Color32::from_rgb(255, 100, 100), // Red - highest priority
                    4 => egui::Color32::from_rgb(255, 165, 0),   // Orange
                    3 => egui::Color32::from_rgb(255, 215, 0),   // Yellow
                    2 => egui::Color32::from_rgb(144, 238, 144), // Light green
                    _ => egui::Color32::GRAY,                    // Gray - lowest
                };

                ui.label(
                    egui::RichText::new(format!("P{}", rec.priority))
                        .small()
                        .strong()
                        .color(priority_color),
                );

                // Conflict indicator
                if proc_rec.has_conflict {
                    ui.label(
                        egui::RichText::new("⚠️").color(egui::Color32::from_rgb(255, 200, 100)),
                    );
                } else {
                    ui.label("•");
                }

                // Category badge (small, subtle)
                ui.label(
                    egui::RichText::new(format!("[{}]", rec.category))
                        .small()
                        .color(egui::Color32::DARK_GRAY),
                );

                ui.label(
                    egui::RichText::new(&rec.parameter)
                        .strong()
                        .color(egui::Color32::from_rgb(242, 97, 63)),
                );
                ui.label("-");
                ui.label(egui::RichText::new(&rec.adjustment).color(egui::Color32::WHITE));
            });

            // Description indented below with improved readability
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.label(
                    egui::RichText::new(&rec.description)
                        .italics()
                        .size(12.0)
                        .color(egui::Color32::GRAY),
                );
            });

            // Show conflict details if present
            if proc_rec.has_conflict && !proc_rec.conflicts.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(15.0);
                    ui.label(
                        egui::RichText::new("⚠️ Conflicts with: ")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(255, 200, 100)),
                    );

                    let conflict_text = proc_rec
                        .conflicts
                        .iter()
                        .map(|c| format!("{} ({})", c.parameter, c.adjustment))
                        .collect::<Vec<_>>()
                        .join(", ");

                    ui.label(
                        egui::RichText::new(conflict_text)
                            .size(11.0)
                            .italics()
                            .color(egui::Color32::from_rgb(255, 200, 100)),
                    );
                });
            }

            ui.add_space(6.0);
        }
    }
}
