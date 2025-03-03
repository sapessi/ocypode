use egui::{
    Align, Color32, CornerRadius, Frame, Id, Image, ImageButton, Layout, RichText, Sense,
    ViewportCommand,
};

use crate::telemetry::TelemetryAnnotation;

use super::{config::AlertsLayout, LiveTelemetryApp, DEFAULT_WINDOW_CORNER_RADIUS};

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
        let mut abs_image = egui::include_image!("../../assets/brake-green.png");
        let mut shift_image = egui::include_image!("../../assets/shift-grey.png");
        let mut wheelspin_image = egui::include_image!("../../assets/wheelspin-green.png");
        let mut trailbrake_steering_image = egui::include_image!("../../assets/steering-grey.png");

        if let Some(back) = self.telemetry_points.back() {
            // brake ABS alert
            if back.brake > 0.4 && !back.abs_active {
                abs_image = egui::include_image!("../../assets/brake-orange.png");
            }
            if back.abs_active {
                abs_image = egui::include_image!("../../assets/brake-red.png");
            }
            // trailbrake steering analyzer
            if back.brake > 0.05 {
                trailbrake_steering_image = egui::include_image!("../../assets/steering-green.png");
            }
            // shift timing alert
            if back.cur_rpm > back.car_shift_ideal_rpm - 100.
                && back.cur_rpm < back.car_shift_ideal_rpm + 100.
            {
                shift_image = egui::include_image!("../../assets/shift-green.png");
            }
            if back.cur_rpm > back.car_shift_ideal_rpm + 100. {
                shift_image = egui::include_image!("../../assets/shift-red.png");
            }
            for annotation in &back.annotations {
                match annotation {
                    TelemetryAnnotation::ShortShifting {
                        gear_change_rpm: _,
                        optimal_rpm: _,
                        is_short_shifting,
                    } => {
                        if *is_short_shifting {
                            shift_image = egui::include_image!("../../assets/shift-orange.png");
                        }
                    }
                    TelemetryAnnotation::TrailbrakeSteering {
                        cur_trailbrake_steering: _,
                        is_excessive_trailbrake_steering,
                    } => {
                        if *is_excessive_trailbrake_steering {
                            trailbrake_steering_image =
                                egui::include_image!("../../assets/steering-red.png");
                        }
                    }
                    TelemetryAnnotation::Wheelspin {
                        avg_rpm_increase_per_gear: _,
                        cur_gear: _,
                        cur_rpm_increase: _,
                        is_wheelspin,
                    } => {
                        if *is_wheelspin {
                            wheelspin_image =
                                egui::include_image!("../../assets/wheelspin-red.png");
                        }
                    }
                    TelemetryAnnotation::Slip {
                        prev_speed: _,
                        cur_speed: _,
                        is_slip: _,
                    } => {
                        _ = self.scrub_slip_alert.update_state(annotation.clone());
                    }
                    TelemetryAnnotation::Scrub {
                        avg_yaw_rate_change: _,
                        cur_yaw_rate_change: _,
                        is_scrubbing: _,
                    } => _ = self.scrub_slip_alert.update_state(annotation.clone()),
                }
            }

            // check previously active alerts that should remain active
        }
        let button_align = match self.app_config.alerts_layout {
            AlertsLayout::Vertical => Align::Center,
            AlertsLayout::Horizontal => Align::LEFT,
        };
        ui.with_layout(Layout::top_down(button_align), |ui| {
            ui.label(RichText::new("ABS").color(Color32::WHITE));
            ui.add(Image::new(abs_image));
        });
        ui.separator();
        ui.with_layout(Layout::top_down(button_align), |ui| {
            ui.label(RichText::new("Shift").color(Color32::WHITE));
            ui.add(Image::new(shift_image));
        });
        ui.separator();
        ui.with_layout(Layout::top_down(button_align), |ui| {
            ui.label(RichText::new("Traction").color(Color32::WHITE));
            ui.add(Image::new(wheelspin_image));
        });
        ui.separator();
        ui.with_layout(Layout::top_down(button_align), |ui| {
            ui.label(RichText::new("Trailbraking").color(Color32::WHITE));
            ui.add(Image::new(trailbrake_steering_image));
        });
        ui.separator();
        self.scrub_slip_alert.show(ui, button_align);
    }
}
