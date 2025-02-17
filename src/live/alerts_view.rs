use egui::{Align, Id, Image, Layout, Sense, ViewportCommand};

use crate::telemetry::TelemetryAnnotation;

use super::LiveTelemetryApp;

impl LiveTelemetryApp {
    pub(crate) fn alerts_view(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::TOP), |ui| {
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

                    if let Some(TelemetryAnnotation::Bool(true)) = back.annotations.get("wheelspin")
                    {
                        wheelspin_image = egui::include_image!("../../assets/wheelspin-red.png");
                    }
                }

                ui.label("ABS");
                ui.add(Image::new(abs_image));
                if ui
                    .interact(ui.max_rect(), Id::new("window-drag"), Sense::drag())
                    .dragged()
                {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                ui.separator();
                ui.label("Shift");
                ui.add(Image::new(shift_image));

                ui.separator();
                ui.label("Traction");
                ui.add(Image::new(wheelspin_image));

                if ui
                    .interact(ui.max_rect(), Id::new("window-drag"), Sense::drag())
                    .dragged()
                {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
            });
        });
    }
}
