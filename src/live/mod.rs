mod alerts_view;
pub(crate) mod config;
mod telemetry_view;

use std::{collections::VecDeque, sync::mpsc::Receiver, time::SystemTime};

use config::AppConfig;
use egui::{
    style::Widgets, Align, Color32, CornerRadius, Image, Layout, RichText, Ui, ViewportBuilder,
    ViewportId, Visuals,
};
use log::error;

use crate::{
    telemetry::{TelemetryAnnotation, TelemetryOutput, TelemetryPoint},
    OcypodeError,
};

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
const DEFAULT_WINDOW_TRANSPARENCY: u8 = 191;

const ALERT_DURATION_MS: u128 = 500;

struct ScrubSlipAlert {
    alert_start_time: SystemTime,
    is_slip: bool,
    is_scrub: bool,
}

impl Default for ScrubSlipAlert {
    fn default() -> Self {
        Self {
            alert_start_time: SystemTime::now(),
            is_slip: false,
            is_scrub: false,
        }
    }
}

impl ScrubSlipAlert {
    fn update_state(&mut self, annotation: TelemetryAnnotation) -> Result<(), OcypodeError> {
        match annotation {
            TelemetryAnnotation::Slip {
                prev_speed: _,
                cur_speed: _,
                is_slip,
            } => {
                self.is_slip = is_slip;
                self.alert_start_time = SystemTime::now();
            }
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: _,
                cur_yaw_rate_change: _,
                is_scrubbing,
            } => {
                self.is_scrub = is_scrubbing;
                self.alert_start_time = SystemTime::now();
            }
            _ => return Err(OcypodeError::InvalidTelemetryAnnotation),
        };
        Ok(())
    }

    fn show(&mut self, ui: &mut Ui, button_align: Align) {
        let mut turn_image = egui::include_image!("../../assets/turn-grey.png");
        let mut text = "slip";
        if SystemTime::now()
            .duration_since(self.alert_start_time)
            .unwrap()
            .as_millis()
            < ALERT_DURATION_MS
        {
            if self.is_slip {
                turn_image = egui::include_image!("../../assets/turn-slip-red.png");
                text = "Slip";
            }
            if self.is_scrub {
                turn_image = egui::include_image!("../../assets/turn-scrub-red.png");
                text = "Scrub";
            }
        }

        ui.with_layout(Layout::top_down(button_align), |ui| {
            ui.label(RichText::new(text).color(Color32::WHITE));
            ui.add(Image::new(turn_image));
        });
    }
}

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
    telemetry_receiver: Receiver<TelemetryOutput>,
    window_size_points: usize,
    telemetry_points: VecDeque<TelemetryPoint>,
    app_config: AppConfig,
    scrub_slip_alert: ScrubSlipAlert,
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
            window_corner_radius: CornerRadius::same(DEFAULT_BUTTON_CORNER_RADIUS),
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
        Self {
            telemetry_receiver,
            window_size_points,
            telemetry_points: VecDeque::new(),
            app_config,
            scrub_slip_alert: ScrubSlipAlert::default(),
        }
    }
}

impl eframe::App for LiveTelemetryApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(e) = self.app_config.save() {
            error!("Error while saving config file: {}", e);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        // read telemetry to window
        let start_refresh = SystemTime::now();
        // consume a few telemetry points and then exist the loop to avoid blocking the UI
        for (cnt, point) in self.telemetry_receiver.try_recv().iter().enumerate() {
            if let TelemetryOutput::DataPoint(point) = point {
                if let Some(last) = self.telemetry_points.back() {
                    if point.point_no < last.point_no {
                        self.telemetry_points.clear()
                    }
                }

                self.telemetry_points.push_back(point.clone());

                while self.telemetry_points.len() >= self.window_size_points
                    && self.telemetry_points.front().is_some()
                {
                    self.telemetry_points.pop_front();
                }

                if cnt > MAX_POINTS_PER_REFRESH
                    || SystemTime::now()
                        .duration_since(start_refresh)
                        .unwrap()
                        .as_millis()
                        >= MAX_TIME_PER_REFRESH_MS
                {
                    break;
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
    }
}
