mod alerts_view;
pub(crate) mod config;
mod telemetry_view;

use std::{collections::VecDeque, sync::mpsc::Receiver, time::SystemTime};

use config::AppConfig;
use egui::{Color32, CornerRadius, ViewportBuilder, ViewportId, Visuals};

use crate::telemetry::TelemetryPoint;

const REFRESH_RATE_MS: usize = 100;
pub(crate) const HISTORY_SECONDS: usize = 5;
const MAX_POINTS_PER_REFRESH: usize = 10;
const MAX_TIME_PER_REFRESH_MS: u128 = 50;

const PALETTE_BLACK: Color32 = Color32::from_rgb(12, 12, 12);
const PALETTE_BROWN: Color32 = Color32::from_rgb(72, 30, 20);
const PALETTE_MAROON: Color32 = Color32::from_rgb(155, 57, 34);
const PALETTE_ORANGE: Color32 = Color32::from_rgb(242, 97, 63);

const DEFAULT_BUTTON_CORNER_RADIUS: u8 = 4;
const DEFAULT_WINDOW_CORNER_RADIUS: u8 = 10;
const DEFAULT_WINDOW_TRANSPARENCY: f32 = 0.75;
const DEFAULT_CONTROLS_TRANSPRENCY: f32 = 0.85;

/// `LiveTelemetryApp` is an application that displays live telemetry data in a graphical interface.
///
/// # Fields
///
/// * `telemetry_receiver` - A receiver for telemetry points.
/// * `window_size_s` - The size of the window in seconds.
/// * `window_size_points` - The size of the window in points.
/// * `telemetry_points` - A deque that stores the telemetry points.
///
/// # Methods
///
/// * `new` - Creates a new instance of `LiveTelemetryApp`.
/// * `update` - Updates the application state and renders the UI.
pub struct LiveTelemetryApp {
    telemetry_receiver: Receiver<TelemetryPoint>,
    window_size_points: usize,
    telemetry_points: VecDeque<TelemetryPoint>,
    app_config: AppConfig,
}

impl LiveTelemetryApp {
    pub fn new(telemetry_receiver: Receiver<TelemetryPoint>, app_config: AppConfig) -> Self {
        let window_size_points = app_config.window_size_s * (1000 / app_config.refresh_rate_ms);
        Self {
            telemetry_receiver,
            window_size_points,
            telemetry_points: VecDeque::new(),
            app_config,
        }
    }
}

impl eframe::App for LiveTelemetryApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(e) = self.app_config.save() {
            println!("Error while saving config file: {}", e);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        let default_visuals = Visuals {
            dark_mode: true,
            hyperlink_color: PALETTE_MAROON,
            faint_bg_color: PALETTE_BLACK,
            extreme_bg_color: PALETTE_BROWN,
            window_corner_radius: CornerRadius::same(10),
            panel_fill: PALETTE_BLACK,
            button_frame: true,
            striped: false,
            ..Default::default()
        };
        ctx.set_visuals(default_visuals.clone());

        // read telemetry to window
        let start_refresh = SystemTime::now();
        // consume a few telemetry points and then exist the loop to avoid blocking the UI
        for point in self.telemetry_receiver.try_recv().iter().enumerate() {
            if let Some(last) = self.telemetry_points.back() {
                if point.1.point_no < last.point_no {
                    self.telemetry_points.clear()
                }
            }

            self.telemetry_points.push_back(point.1.clone());

            while self.telemetry_points.len() >= self.window_size_points
                && self.telemetry_points.front().is_some()
            {
                self.telemetry_points.pop_front();
            }

            if point.0 > MAX_POINTS_PER_REFRESH
                || SystemTime::now()
                    .duration_since(start_refresh)
                    .unwrap()
                    .as_millis()
                    >= MAX_TIME_PER_REFRESH_MS
            {
                break;
            }
        }

        self.telemetry_view(ctx, _frame);

        // open separate alerts viewport
        if self.app_config.show_alerts {
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("alerts"),
                ViewportBuilder::default()
                    .with_always_on_top()
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_position(self.app_config.alert_window_position.clone())
                    .with_inner_size(self.app_config.alerts_layout.window_size()),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );
                    ctx.set_visuals(default_visuals.clone());
                    self.alerts_view(ctx, _frame);
                },
            );
        }
    }
}
