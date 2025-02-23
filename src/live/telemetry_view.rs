use std::sync::Arc;

use egui::{Color32, Id, ImageButton, Layout, Sense, Vec2b, ViewportCommand};
use egui_plot::{Line, PlotPoints};

use super::LiveTelemetryApp;

impl LiveTelemetryApp {
    pub(crate) fn telemetry_view(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("settings")
            .min_height(30.)
            .show(ctx, |ui| {
                if ui
                    .interact(ui.max_rect(), Id::new("window-drag"), Sense::drag())
                    .dragged()
                {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    // icons from https://remixicon.com/
                    ui.add(ImageButton::new(egui::include_image!(
                        "../../assets/tools-fill.png"
                    )));
                    if ui
                        .add(ImageButton::new(egui::include_image!(
                            "../../assets/alert-fill.png"
                        )))
                        .clicked()
                    {
                        self.app_config.show_alerts = !self.app_config.show_alerts;
                    };

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(ImageButton::new(egui::include_image!(
                                "../../assets/close-circle-fill.png"
                            )))
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
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
                steering_vec.push([p.0 as f64, p.1.steering as f64]);
                true
            });

            let throttle_points = PlotPoints::new(throttle_vec);
            let brake_points = PlotPoints::new(brake_vec);
            //let steering_points = PlotPoints::new(steering_vec);

            plot.show(ui, |plot_ui| {
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
                                stroke_shade(Color32::YELLOW, Color32::RED, (point.y / 100.) as f32)
                            }),
                            true,
                        )
                        .color(Color32::RED)
                        .fill(0.)
                        .name("Brake"),
                );
            });
        });
        // make it always repaint. TODO: can we slow down here?
        ctx.request_repaint();
    }
}

fn stroke_shade(start: Color32, end: Color32, y: f32) -> Color32 {
    Color32::from_rgb(
        u8::try_from((start.r() as f32 + y * (end.r() as f32 - start.r() as f32)) as u32)
            .map_err(|e| println!("{}", e))
            .unwrap(),
        u8::try_from((start.g() as f32 + y * (end.g() as f32 - start.g() as f32)) as u32)
            .map_err(|e| println!("{}", e))
            .unwrap(),
        u8::try_from((start.b() as f32 + y * (end.b() as f32 - start.b() as f32)) as u32)
            .map_err(|e| println!("{}", e))
            .unwrap(),
    )
}
