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
    fn show_recommendations(&self, ui: &mut egui::Ui) {
        use std::collections::HashMap;

        // Get all recommendations for confirmed findings
        // This updates in real-time as the user confirms/unconfirms findings
        let all_recommendations = self.setup_assistant.get_recommendations();

        // If no confirmed findings, show a message
        if all_recommendations.is_empty() {
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
        ui.label(
            egui::RichText::new("Based on confirmed issues")
                .size(12.0)
                .color(egui::Color32::GRAY),
        );
        ui.add_space(12.0);

        // Group recommendations by category
        let mut by_category: HashMap<String, Vec<&crate::setup_assistant::SetupRecommendation>> =
            HashMap::new();

        for rec in &all_recommendations {
            let category_key = format!("{}", rec.category);
            by_category.entry(category_key).or_default().push(rec);
        }

        // Sort categories for consistent display
        let mut categories: Vec<_> = by_category.keys().cloned().collect();
        categories.sort();

        // Display recommendations grouped by category with improved layout
        for category in categories {
            if let Some(recs) = by_category.get(&category) {
                // Category header with visual emphasis
                ui.label(egui::RichText::new(&category).strong().size(14.0));
                ui.add_space(6.0);

                // Display each recommendation in this category
                for rec in recs {
                    // Parameter and adjustment on one line with improved styling
                    ui.horizontal(|ui| {
                        ui.label("•");
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

                    ui.add_space(4.0);
                }

                ui.add_space(10.0);
            }
        }
    }
}
