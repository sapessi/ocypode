use std::sync::Arc;

use egui::{Color32, CornerRadius, Frame, Id, ImageButton, Layout, Sense, Vec2b, ViewportCommand};
use egui_plot::{Line, PlotPoints};
use log::debug;

use super::{
    LiveTelemetryApp, DEFAULT_BUTTON_CORNER_RADIUS, DEFAULT_WINDOW_CORNER_RADIUS, PALETTE_ORANGE,
};

impl LiveTelemetryApp {
    pub(crate) fn telemetry_view(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("settings")
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
                        self.app_config.telemetry_window_position = outer_rect.min.into();
                    }
                }
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(10.);
                    // icons from https://remixicon.com/
                    ui.add(
                        ImageButton::new(egui::include_image!("../../assets/tools-fill.png"))
                            .corner_radius(DEFAULT_BUTTON_CORNER_RADIUS),
                    );
                    if ui
                        .add(
                            ImageButton::new(egui::include_image!("../../assets/alert-fill.png"))
                                .corner_radius(DEFAULT_BUTTON_CORNER_RADIUS),
                        )
                        .clicked()
                    {
                        self.app_config.show_alerts = !self.app_config.show_alerts;
                    };

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.);
                        if ui
                            .add(
                                ImageButton::new(egui::include_image!(
                                    "../../assets/close-circle-fill.png"
                                ))
                                .corner_radius(DEFAULT_BUTTON_CORNER_RADIUS),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::new().corner_radius(CornerRadius {
                sw: DEFAULT_WINDOW_CORNER_RADIUS,
                se: DEFAULT_WINDOW_CORNER_RADIUS,
                ..Default::default()
            }))
            .show(ctx, |ui| {
                let plot = egui_plot::Plot::new("measurements")
                    .allow_drag(false)
                    .allow_scroll(false)
                    .allow_zoom(false)
                    .include_x(0.)
                    .include_x(self.window_size_points as f64)
                    .include_y(0.)
                    .include_y(100.)
                    .auto_bounds(Vec2b::new(true, false))
                    .show_grid(false);
                let mut throttle_vec = Vec::<[f64; 2]>::new();
                let mut brake_vec = Vec::<[f64; 2]>::new();
                let mut steering_vec = Vec::<[f64; 2]>::new();

                self.telemetry_points.iter().enumerate().all(|p| {
                    throttle_vec.push([p.0 as f64, p.1.throttle as f64 * 100.]);
                    brake_vec.push([p.0 as f64, p.1.brake as f64 * 100.]);
                    steering_vec.push([p.0 as f64, 50. + 50. * p.1.steering_pct as f64]);
                    true
                });

                let throttle_points = PlotPoints::new(throttle_vec);
                let brake_points = PlotPoints::new(brake_vec);
                let steering_points = PlotPoints::new(steering_vec);

                plot.show_background(false).show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(throttle_points)
                            .color(Color32::GREEN)
                            .fill(0.)
                            .name("Throttle"),
                    );
                    plot_ui.line(
                        Line::new(brake_points)
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
                            .fill(0.)
                            .name("Brake"),
                    );
                    plot_ui.line(
                        Line::new(steering_points)
                            .color(Color32::LIGHT_GRAY)
                            .name("Steering"),
                    );
                });
            });
        // make it always repaint. TODO: can we slow down here?
        ctx.request_repaint();
    }
}

fn stroke_shade(start: Color32, end: Color32, y: f32) -> Color32 {
    Color32::from_rgb(
        u8::try_from(
            (start.r() as f32 + y * (end.r() as f32 - start.r() as f32)).clamp(0., 255.) as u32,
        )
        .map_err(|e| debug!("Error interpolating colors: {}", e))
        .unwrap(),
        u8::try_from(
            (start.g() as f32 + y * (end.g() as f32 - start.g() as f32)).clamp(0., 255.) as u32,
        )
        .map_err(|e| debug!("Error interpolating colors: {}", e))
        .unwrap(),
        u8::try_from(
            (start.b() as f32 + y * (end.b() as f32 - start.b() as f32)).clamp(0., 255.) as u32,
        )
        .map_err(|e| debug!("Error interpolating colors: {}", e))
        .unwrap(),
    )
}
