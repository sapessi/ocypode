use egui::{Align, Id, Image, ImageButton, Layout, Sense, ViewportCommand};

use crate::telemetry::TelemetryAnnotation;

use super::{config::AlertsLayout, LiveTelemetryApp};

impl LiveTelemetryApp {
    pub(crate) fn alerts_view(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Controls")
            .min_height(30.)
            .show(ctx, |ui| {
                let drag_sense = ui.interact(ui.max_rect(), Id::new("window-drag"), Sense::drag());
                if drag_sense.dragged() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag_sense.drag_stopped() {
                    if let Some(outer_rect) = ui.input(|is| is.viewport().outer_rect) {
                        self.app_config.alert_window_position = outer_rect.min.into();
                    };
                }
                match self.app_config.alerts_layout {
                    AlertsLayout::Vertical => {
                        if ui
                            .add(ImageButton::new(egui::include_image!(
                                "../../assets/layout-horizontal-fill.png"
                            )))
                            .clicked()
                        {
                            self.app_config.alerts_layout = AlertsLayout::Horizontal;
                        }
                    }
                    AlertsLayout::Horizontal => {
                        if ui
                            .add(ImageButton::new(egui::include_image!(
                                "../../assets/layout-vertical-fill.png"
                            )))
                            .clicked()
                        {
                            self.app_config.alerts_layout = AlertsLayout::Vertical;
                        }
                    }
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| match self.app_config.alerts_layout {
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
        let mut abs_image = egui::include_image!("../../assets/brake-green.png");
        let mut shift_image = egui::include_image!("../../assets/shift-orange.png");
        let mut wheelspin_image = egui::include_image!("../../assets/wheelspin-green.png");
        if let Some(back) = self.telemetry_points.back() {
            if back.brake > 0.4 && !back.abs_active {
                abs_image = egui::include_image!("../../assets/brake-orange.png");
            }
            if back.abs_active {
                abs_image = egui::include_image!("../../assets/brake-red.png");
            }

            if back.cur_rpm > back.car_shift_ideal_rpm - 100.
                && back.cur_rpm < back.car_shift_ideal_rpm + 100.
            {
                shift_image = egui::include_image!("../../assets/shift-green.png");
            }

            if let Some(TelemetryAnnotation::Bool(true)) = back.annotations.get("wheelspin") {
                wheelspin_image = egui::include_image!("../../assets/wheelspin-red.png");
            }
        }

        ui.label("ABS");
        ui.add(Image::new(abs_image));
        ui.separator();
        ui.label("Shift");
        ui.add(Image::new(shift_image));

        ui.separator();
        ui.label("Traction");
        ui.add(Image::new(wheelspin_image));
    }
}
