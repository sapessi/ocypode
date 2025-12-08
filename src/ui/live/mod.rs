mod alerts_view;
pub(crate) mod config;
mod setup_window;
pub(crate) mod telemetry_view;

use std::{collections::VecDeque, sync::mpsc::Receiver, time::SystemTime};

use config::AppConfig;
use egui::{Color32, ViewportBuilder, ViewportId, Visuals, style::Widgets};
use log::error;

use crate::setup_assistant::SetupAssistant;
use crate::telemetry::{TelemetryData, TelemetryOutput};

use super::ScrubSlipAlert;

const REFRESH_RATE_MS: usize = 100;
pub(crate) const HISTORY_SECONDS: usize = 5;
const MAX_POINTS_PER_REFRESH: usize = 10;
const MAX_TIME_PER_REFRESH_MS: u128 = 50;

pub(crate) const PALETTE_BLACK: Color32 = Color32::from_rgb(12, 12, 12);
pub(crate) const PALETTE_BROWN: Color32 = Color32::from_rgb(72, 30, 20);
pub(crate) const PALETTE_MAROON: Color32 = Color32::from_rgb(155, 57, 34);
pub(crate) const PALETTE_ORANGE: Color32 = Color32::from_rgb(242, 97, 63);

const DEFAULT_BUTTON_CORNER_RADIUS: u8 = 4;
const DEFAULT_WINDOW_CORNER_RADIUS: u8 = 10;
const DEFAULT_WINDOW_TRANSPARENCY: u8 = 191;

/// `LiveTelemetryApp` is an application that displays live telemetry data in a graphical interface.
///
/// # Fields
///
/// * `telemetry_receiver` - A receiver for telemetry points.
/// * `window_size_s` - The size of the window in seconds.
/// * `window_size_points` - The size of the window in points.
/// * `telemetry_points` - A deque that stores the telemetry points.
/// * `setup_assistant` - The setup assistant for analyzing telemetry and providing recommendations.
///
/// # Methods
///
/// * `new` - Creates a new instance of `LiveTelemetryApp`.
/// * `update` - Updates the application state and renders the UI.
pub struct LiveTelemetryApp {
    telemetry_receiver: Receiver<TelemetryOutput>,
    window_size_points: usize,
    telemetry_points: VecDeque<TelemetryData>,
    app_config: AppConfig,
    scrub_slip_alert: ScrubSlipAlert,
    setup_assistant: SetupAssistant,
}

impl LiveTelemetryApp {
    pub fn new(
        telemetry_receiver: Receiver<TelemetryOutput>,
        app_config: AppConfig,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        let default_visuals = Visuals {
            dark_mode: true,
            hyperlink_color: PALETTE_MAROON,
            faint_bg_color: PALETTE_BLACK,
            extreme_bg_color: PALETTE_BROWN,
            panel_fill: PALETTE_BLACK,
            button_frame: true,
            window_fill: Color32::from_rgba_premultiplied(
                PALETTE_BLACK.r(),
                PALETTE_BLACK.g(),
                PALETTE_BLACK.b(),
                DEFAULT_WINDOW_TRANSPARENCY,
            ),
            widgets: Widgets::dark(),
            striped: false,
            ..Default::default()
        };
        cc.egui_ctx.set_visuals(default_visuals);

        let window_size_points = app_config.window_size_s * (1000 / app_config.refresh_rate_ms);

        // Create setup assistant and restore persisted state
        let mut setup_assistant = SetupAssistant::new();
        setup_assistant.restore_findings(app_config.setup_assistant_findings.clone());
        setup_assistant
            .restore_confirmed_findings(app_config.setup_assistant_confirmed_findings.clone());

        Self {
            telemetry_receiver,
            window_size_points,
            telemetry_points: VecDeque::new(),
            app_config,
            scrub_slip_alert: ScrubSlipAlert::default(),
            setup_assistant,
        }
    }
}

impl eframe::App for LiveTelemetryApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save setup assistant state to config before exiting
        self.app_config.setup_assistant_findings =
            self.setup_assistant.get_findings_for_persistence().clone();
        self.app_config.setup_assistant_confirmed_findings = self
            .setup_assistant
            .get_confirmed_findings_for_persistence()
            .clone();

        if let Err(e) = self.app_config.save() {
            error!("Error while saving config file: {}", e);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        // read telemetry to window
        let start_refresh = SystemTime::now();
        // consume a few telemetry points and then exit the loop to avoid blocking the UI
        let mut points_processed = 0;
        while let Ok(output) = self.telemetry_receiver.try_recv() {
            match output {
                TelemetryOutput::DataPoint(point) => {
                    if let Some(last) = self.telemetry_points.back() {
                        if point.point_no < last.point_no {
                            self.telemetry_points.clear()
                        }
                    }

                    // Process telemetry through setup assistant
                    self.setup_assistant.process_telemetry(&point);

                    self.telemetry_points.push_back(*point);

                    // Remove old points if we exceed window size
                    if self.telemetry_points.len() > self.window_size_points {
                        self.telemetry_points.pop_front();
                    }

                    points_processed += 1;

                    // Exit if we've processed enough points or taken too long
                    if points_processed > MAX_POINTS_PER_REFRESH
                        || SystemTime::now()
                            .duration_since(start_refresh)
                            .unwrap()
                            .as_millis()
                            >= MAX_TIME_PER_REFRESH_MS
                    {
                        break;
                    }
                }
                TelemetryOutput::SessionChange(_session_info) => {
                    // Clear setup assistant findings when session changes
                    self.setup_assistant.clear_session();
                }
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
                    self.alerts_view(ctx, _frame);
                },
            );
        }

        // open separate setup window viewport
        if self.app_config.show_setup_window {
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("setup_assistant"),
                ViewportBuilder::default()
                    .with_always_on_top()
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_position(self.app_config.setup_window_position.clone())
                    .with_inner_size([400.0, 600.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );
                    self.setup_window(ctx, _frame);
                },
            );
        }
    }
}
