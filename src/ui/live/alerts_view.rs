use egui::{Align, Button, CornerRadius, Frame, Id, Layout, Sense, ViewportCommand};

use crate::ui::{Alert, DefaultAlert};

use super::{DEFAULT_WINDOW_CORNER_RADIUS, LiveTelemetryApp, config::AlertsLayout};

impl LiveTelemetryApp {
    pub(crate) fn alerts_view(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Controls")
            .min_height(30.)
            .frame(Frame::new().corner_radius(CornerRadius {
                nw: DEFAULT_WINDOW_CORNER_RADIUS,
                ne: DEFAULT_WINDOW_CORNER_RADIUS,
                ..Default::default()
            }))
            .show(ctx, |ui| {
                let drag_sense = ui.interact(ui.max_rect(), Id::new("window-drag"), Sense::drag());
                if drag_sense.dragged() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag_sense.drag_stopped()
                    && let Some(outer_rect) = ui.input(|is| is.viewport().outer_rect)
                {
                    self.app_config.alert_window_position = outer_rect.min.into();
                };

                match self.app_config.alerts_layout {
                    AlertsLayout::Vertical => {
                        if ui
                            .add(Button::image(egui::include_image!(
                                "../../../assets/layout-horizontal-fill.png"
                            )))
                            .clicked()
                        {
                            self.app_config.alerts_layout = AlertsLayout::Horizontal;
                        }
                    }
                    AlertsLayout::Horizontal => {
                        if ui
                            .add(Button::image(egui::include_image!(
                                "../../../assets/layout-vertical-fill.png"
                            )))
                            .clicked()
                        {
                            self.app_config.alerts_layout = AlertsLayout::Vertical;
                        }
                    }
                }
            });
        egui::CentralPanel::default()
            .frame(Frame::new().corner_radius(CornerRadius {
                sw: DEFAULT_WINDOW_CORNER_RADIUS,
                se: DEFAULT_WINDOW_CORNER_RADIUS,
                ..Default::default()
            }))
            .show(ctx, |ui| match self.app_config.alerts_layout {
                AlertsLayout::Vertical => {
                    ui.with_layout(Layout::top_down(Align::TOP), |ui| {
                        self.show_alerts(ui);
                    });
                }
                AlertsLayout::Horizontal => {
                    ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                        self.show_alerts(ui);
                    });
                }
            });
    }

    fn show_alerts(&mut self, ui: &mut egui::Ui) {
        // load warning based on telemetry data
        let mut abs_alert = DefaultAlert::abs();
        let mut shift_alert = DefaultAlert::shift();
        let mut traction_alert = DefaultAlert::traction();
        let mut trailbrake_steering_alert = DefaultAlert::trailbrake_steering();

        if let Some(telemetry) = self.telemetry_points.back() {
            let _ = abs_alert.update_state(telemetry);
            let _ = shift_alert.update_state(telemetry);
            let _ = traction_alert.update_state(telemetry);
            let _ = trailbrake_steering_alert.update_state(telemetry);
            let _ = self.scrub_slip_alert.update_state(telemetry);
        }

        let button_align = match self.app_config.alerts_layout {
            AlertsLayout::Vertical => Align::Center,
            AlertsLayout::Horizontal => Align::LEFT,
        };
        abs_alert.show(ui, button_align);
        ui.separator();
        shift_alert.show(ui, button_align);
        ui.separator();
        traction_alert.show(ui, button_align);
        ui.separator();
        trailbrake_steering_alert.show(ui, button_align);
        ui.separator();
        self.scrub_slip_alert.show(ui, button_align);
    }
}
