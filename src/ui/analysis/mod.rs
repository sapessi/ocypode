use std::{path::PathBuf, sync::Arc};

use egui::{
    Align, Color32, Direction, Frame, Label, Layout, Margin, RichText, Ui, Vec2b, Visuals,
    style::Widgets,
};
use egui_dropdown::DropDownBox;
use egui_plot::{Legend, Line, PlotPoints, Points};
use itertools::Itertools;

use crate::{
    OcypodeError,
    telemetry::{SessionInfo, TelemetryAnnotation, TelemetryOutput, SerializableTelemetry},
    ui::live::{PALETTE_BLACK, PALETTE_BROWN, PALETTE_MAROON, PALETTE_ORANGE},
};

use super::{Alert, DefaultAlert, ScrubSlipAlert, stroke_shade};

#[derive(Default, Clone, Debug)]
struct TelemetryFile {
    sessions: Vec<Session>,
}

#[derive(Default, Clone, Debug)]
struct Lap {
    telemetry: Vec<SerializableTelemetry>,
}

#[derive(Default, Clone, Debug)]
struct Session {
    info: SessionInfo,
    laps: Vec<Lap>,
}

#[derive(Clone)]
enum UiState {
    Loading,
    Error { message: String },
    Display { session: Session },
}

pub(crate) struct TelemetryAnalysisApp<'file> {
    source_file: &'file PathBuf,
    ui_state: UiState,
    data: Option<TelemetryFile>,
    selected_session: String,
    selected_lap: String,
    comparison_lap: String,
    selected_annotation_content: String,
    selected_x: Option<usize>,
}

impl<'file> TelemetryAnalysisApp<'file> {
    pub(crate) fn from_file(input: &'file PathBuf, cc: &eframe::CreationContext<'_>) -> Self {
        let default_visuals = Visuals {
            dark_mode: true,
            hyperlink_color: PALETTE_MAROON,
            faint_bg_color: PALETTE_BLACK,
            extreme_bg_color: PALETTE_BROWN,
            panel_fill: PALETTE_BLACK,
            button_frame: true,
            window_fill: PALETTE_BLACK,
            widgets: Widgets::dark(),
            striped: false,
            ..Default::default()
        };
        cc.egui_ctx.set_visuals(default_visuals);
        Self {
            source_file: input,
            ui_state: UiState::Loading,
            data: None,
            selected_session: "".to_string(),
            selected_lap: "".to_string(),
            comparison_lap: "".to_string(),
            selected_annotation_content: "".to_string(),
            selected_x: None,
        }
    }

    fn show_selectors(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            let sessions = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .map(|i| i.info.track_name.as_str());
            ui.label(RichText::new("Session: ").color(Color32::WHITE));
            ui.add(
                DropDownBox::from_iter(
                    sessions,
                    "session_dropbox",
                    &mut self.selected_session,
                    |ui, text| ui.selectable_label(false, text),
                )
                .filter_by_input(false),
            );

            if let Some(selected_session) = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .find(|p| p.info.track_name == self.selected_session)
            {
                ui.separator();
                ui.label(RichText::new("Lap: ").color(Color32::WHITE));
                let laps_iter = (0..selected_session.laps.len())
                    .map(|l| l.to_string())
                    .collect_vec();
                ui.add(
                    DropDownBox::from_iter(
                        laps_iter,
                        "lap_dropbox",
                        &mut self.selected_lap,
                        |ui, text| ui.selectable_label(false, text),
                    )
                    .filter_by_input(false),
                );
            }

            if let Some(selected_session) = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .find(|p| p.info.track_name == self.selected_session)
            {
                ui.separator();
                ui.label(RichText::new("Comparison lap: ").color(Color32::WHITE));
                let laps_iter = (0..selected_session.laps.len())
                    .map(|l| l.to_string())
                    .collect_vec();
                ui.add(
                    DropDownBox::from_iter(
                        laps_iter,
                        "comparison_lap_dropbox",
                        &mut self.comparison_lap,
                        |ui, text| ui.selectable_label(false, text),
                    )
                    .filter_by_input(false),
                );
            }
        });
    }

    fn show_telemetry_chart(&mut self, selected_lap: usize, session: &Session, ui: &mut Ui) {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            let plot = egui_plot::Plot::new("measurements");
            //println!("Selected lap = {}", selected_lap);
            if let Some(lap) = session.laps.get(selected_lap) {
                let mut throttle_vec = Vec::<[f64; 2]>::new();
                let mut brake_vec = Vec::<[f64; 2]>::new();
                let mut steering_vec = Vec::<[f64; 2]>::new();
                let mut annotations_vec = Vec::<[f64; 2]>::new();

                lap.telemetry.iter().enumerate().all(|p| {
                    let throttle = p.1.throttle.unwrap_or(0.0);
                    let brake = p.1.brake.unwrap_or(0.0);
                    let steering_pct = p.1.steering_pct.unwrap_or(0.0);
                    throttle_vec.push([p.0 as f64, throttle as f64 * 100.]);
                    brake_vec.push([p.0 as f64, brake as f64 * 100.]);
                    steering_vec.push([p.0 as f64, 50. + 50. * steering_pct as f64]);
                    if !p.1.annotations.is_empty() {
                        annotations_vec.push([p.0 as f64, 101.]);
                    }
                    true
                });

                let throttle_points = PlotPoints::new(throttle_vec);
                let brake_points = PlotPoints::new(brake_vec);
                let steering_points = PlotPoints::new(steering_vec);
                let annotation_points = PlotPoints::new(annotations_vec);

                let plot_response = plot
                    .show_background(false)
                    .legend(Legend::default())
                    .include_y(0.)
                    .include_y(150.)
                    .include_x(0.)
                    .include_x(250.) // TODO: make this dynamic based on window size
                    .auto_bounds(Vec2b::new(false, false))
                    .show(ui, |plot_ui| {
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
                        plot_ui.points(
                            Points::new(annotation_points)
                                .color(Color32::BLUE)
                                .radius(10.)
                                .name("Annotation"),
                        );

                        if !self.comparison_lap.is_empty() {
                            if let Some(comparison_lap) = session
                                .laps
                                .get(self.comparison_lap.parse::<usize>().unwrap())
                            {
                                let comparison_throttle_points = PlotPoints::new(
                                    comparison_lap
                                        .telemetry
                                        .iter()
                                        .enumerate()
                                        .map(|t| {
                                            let throttle = t.1.throttle.unwrap_or(0.0);
                                            [t.0 as f64, throttle as f64 * 100.]
                                        })
                                        .collect(),
                                );
                                let comparison_brake_points = PlotPoints::new(
                                    comparison_lap
                                        .telemetry
                                        .iter()
                                        .enumerate()
                                        .map(|t| {
                                            let brake = t.1.brake.unwrap_or(0.0);
                                            [t.0 as f64, brake as f64 * 100.]
                                        })
                                        .collect(),
                                );
                                let comparison_steering_points = PlotPoints::new(
                                    comparison_lap
                                        .telemetry
                                        .iter()
                                        .enumerate()
                                        .map(|t| {
                                            let steering_pct = t.1.steering_pct.unwrap_or(0.0);
                                            [t.0 as f64, 50. + 50. * steering_pct as f64]
                                        })
                                        .collect(),
                                );

                                plot_ui.line(
                                    Line::new(comparison_throttle_points)
                                        .color(Color32::DARK_GREEN)
                                        .name("Comparison Throttle"),
                                );
                                plot_ui.line(
                                    Line::new(comparison_brake_points)
                                        .color(Color32::DARK_RED)
                                        .name("Comparison Brake"),
                                );
                                plot_ui.line(
                                    Line::new(comparison_steering_points)
                                        .color(Color32::DARK_GRAY.gamma_multiply(0.3))
                                        .name("Comparison Steering"),
                                );
                            }
                        }
                    });
                if plot_response.response.clicked() {
                    if let Some(mouse_pos) = plot_response.response.interact_pointer_pos() {
                        self.selected_annotation_content = "".to_string();
                        self.selected_x = Some(
                            plot_response
                                .transform
                                .value_from_position(mouse_pos)
                                .x
                                .floor() as usize,
                        );
                    }
                }
            }
        });
    }
}

impl eframe::App for TelemetryAnalysisApp<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        let cur_ui_state = self.ui_state.clone();
        match cur_ui_state {
            UiState::Loading => {
                if self.data.is_none() {
                    let telemetry_load_result = load_telemetry_jsonl(self.source_file);
                    if telemetry_load_result.is_err() {
                        self.ui_state = UiState::Error {
                            message: format!(
                                "Could not load telemetry: {}",
                                telemetry_load_result.err().unwrap()
                            ),
                        };
                        return;
                    }
                    self.data = Some(telemetry_load_result.unwrap());
                    self.ui_state = UiState::Display {
                        session: self
                            .data
                            .as_ref()
                            .unwrap()
                            .sessions
                            .first()
                            .unwrap()
                            .clone(),
                    }
                }
            }
            UiState::Display { session } => {
                egui::TopBottomPanel::top("SessionSelector")
                    .frame(
                        Frame::default()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::same(5)),
                    )
                    .show(ctx, |local_ui| {
                        self.show_selectors(local_ui);
                    });
                egui::SidePanel::right("AnnotationDetail")
                    .frame(
                        Frame::default()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::same(5)),
                    )
                    .resizable(false)
                    .min_width(ctx.available_rect().width() * 0.3)
                    .max_width(ctx.available_rect().height() / 7.)
                    .show(ctx, |local_ui| {
                        if let Ok(selected_lap) = self.selected_lap.parse::<usize>() {
                            if let Some(x_point) = self.selected_x {
                                if let Some(lap) = session.laps.get(selected_lap) {
                                    if let Some(telemetry) = lap.telemetry.get(x_point) {
                                        let mut abs_alert = DefaultAlert::abs().button();
                                        let mut shift_alert = DefaultAlert::shift().button();
                                        let mut traction_alert = DefaultAlert::traction().button();
                                        let mut trailbrake_steering_alert = DefaultAlert::trailbrake_steering().button();
                                        let mut slip_alert = ScrubSlipAlert::default().button();

                                        let _ = abs_alert.update_state(telemetry);
                                        let _ = shift_alert.update_state(telemetry);
                                        let _ = traction_alert.update_state(telemetry);
                                        let _ = trailbrake_steering_alert.update_state(telemetry);
                                        let _ = slip_alert.update_state(telemetry);

                                        local_ui.with_layout(Layout::top_down(Align::Center), |ui| {
                                            if abs_alert.show(ui, Align::Center).clicked() {
                                                let brake = telemetry.brake.unwrap_or(0.0);
                                                self.selected_annotation_content = format!("brake force: {:.2}", brake);
                                            };
                                            ui.separator();
                                            if shift_alert.show(ui, Align::Center).clicked() {
                                                if let Some(TelemetryAnnotation::ShortShifting { gear_change_rpm, optimal_rpm, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::ShortShifting { .. })) {
                                                        let cur_gear = telemetry.gear.unwrap_or(0);
                                                        self.selected_annotation_content = format!(
                                                            "From gear: {}\nTo gear: {}\nIdeal RPM: {}\nActual RPM: {}",
                                                            cur_gear - 1,
                                                            cur_gear,
                                                            optimal_rpm,
                                                            gear_change_rpm
                                                        )
                                                }
                                            }
                                            ui.separator();
                                            if traction_alert.show(ui, Align::Center).clicked() {
                                                if let Some(TelemetryAnnotation::Wheelspin { avg_rpm_increase_per_gear, cur_gear, cur_rpm_increase, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::Wheelspin { .. })) {
                                                        self.selected_annotation_content = format!(
                                                            "Gear: {}\nRPM increase: {:.1}\np90 RPM increase: {:.1}\nRPM increase per gear:\n{}",
                                                            cur_gear,
                                                            cur_rpm_increase,
                                                            avg_rpm_increase_per_gear.get(cur_gear).unwrap(),
                                                            serde_json::to_string_pretty(avg_rpm_increase_per_gear).unwrap()
                                                        );
                                                }
                                            }
                                            ui.separator();
                                            if trailbrake_steering_alert.show(ui, Align::Center).clicked() {
                                                if let Some(TelemetryAnnotation::TrailbrakeSteering { cur_trailbrake_steering, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::TrailbrakeSteering { .. })) {
                                                        let steering = telemetry.steering.unwrap_or(0.0);
                                                        self.selected_annotation_content = format!(
                                                            "Steering: {:.2}%\nSteering angle (rad): {}",
                                                            cur_trailbrake_steering,
                                                            steering
                                                        );
                                                }
                                            }
                                            ui.separator();
                                            if slip_alert.show(ui, Align::Center).clicked() {
                                                if let Some(TelemetryAnnotation::Scrub { avg_yaw_rate_change, cur_yaw_rate_change, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::Scrub { .. })) {
                                                        let steering = telemetry.steering.unwrap_or(0.0);
                                                        let speed = telemetry.speed_mps.unwrap_or(0.0);
                                                        self.selected_annotation_content = format!(
                                                            "Yaw change: {:.2}\nAvg yaw change: {:.2}\nSteering (rad): {:.2}\nSpeed: {:.2}",
                                                            cur_yaw_rate_change,
                                                            avg_yaw_rate_change,
                                                            steering,
                                                            speed
                                                        );
                                                }
                                                if let Some(TelemetryAnnotation::Slip { prev_speed, cur_speed, .. }) =
                                                    telemetry.annotations.iter().find(|p| matches!(p, TelemetryAnnotation::Slip { .. })) {
                                                        let throttle = telemetry.throttle.unwrap_or(0.0);
                                                        let steering = telemetry.steering.unwrap_or(0.0);
                                                        self.selected_annotation_content = format!(
                                                            "Speed: {:.2}\nPrev speed: {:.2}\nThrottle %: {:.2}%\nSteering (rad): {:.2}%",
                                                            cur_speed,
                                                            prev_speed,
                                                            throttle,
                                                            steering
                                                        );
                                                }
                                            }
                                        });

                                        local_ui.add(
                                            Label::new(RichText::new(self.selected_annotation_content.clone()).color(Color32::WHITE))
                                        );
                                    }
                                }
                            } else {
                                local_ui.with_layout(
                                    Layout::centered_and_justified(Direction::TopDown),
                                    |ui| {
                                        ui.label(
                                            RichText::new("No telemetry point selected")
                                                .color(Color32::WHITE)
                                                .strong(),
                                        );
                                    },
                                );
                            }
                        }
                    });
                egui::CentralPanel::default()
                    .frame(
                        Frame::default()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::same(5)),
                    )
                    .show(ctx, |local_ui| {
                        if let Ok(selected_lap) = self.selected_lap.parse::<usize>() {
                            self.show_telemetry_chart(selected_lap, &session, local_ui);
                        }
                    });
            }
            UiState::Error { message } => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading(RichText::new(message).color(Color32::RED).strong());
                });
            }
        }
        ctx.request_repaint();
    }
}

/// Detects if a telemetry file uses the legacy TelemetryPoint format
/// by attempting to parse the first line as a raw JSON value and checking
/// for the presence of legacy-specific fields.
fn is_legacy_format(source_file: &PathBuf) -> bool {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    // Try to read the first line of the file
    let file = match File::open(source_file) {
        Ok(f) => f,
        Err(_) => return false,
    };
    
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    
    if reader.read_line(&mut first_line).is_err() {
        return false;
    }
    
    // Try to parse as JSON
    let json_value: serde_json::Value = match serde_json::from_str(&first_line) {
        Ok(v) => v,
        Err(_) => return false,
    };
    
    // Check if it's a DataPoint variant
    if let Some(obj) = json_value.get("DataPoint") {
        // Legacy format has fields like "cur_gear", "cur_rpm", "cur_speed", "lap_dist"
        // New format has "gear", "engine_rpm", "speed_mps", "lap_distance"
        let has_legacy_fields = obj.get("cur_gear").is_some()
            || obj.get("cur_rpm").is_some()
            || obj.get("cur_speed").is_some()
            || obj.get("lap_dist").is_some()
            || obj.get("car_shift_ideal_rpm").is_some();
        
        let has_new_fields = obj.get("game_source").is_some();
        
        // If it has legacy fields and doesn't have new fields, it's legacy format
        return has_legacy_fields && !has_new_fields;
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_legacy_format_detection() {
        // Create a temporary file with legacy format
        let mut legacy_file = NamedTempFile::new().unwrap();
        writeln!(
            legacy_file,
            r#"{{"DataPoint":{{"point_no":0,"point_epoch":1234567890,"lap_dist":100.0,"lap_dist_pct":0.5,"last_lap_time_s":90.0,"best_lap_time_s":88.0,"car_shift_ideal_rpm":7000.0,"cur_gear":3,"cur_rpm":5000.0,"cur_speed":50.0,"lap_no":1,"throttle":0.8,"brake":0.0,"steering":0.1,"steering_pct":0.05,"abs_active":false,"lat":0.0,"lon":0.0,"lat_accel":0.0,"lon_accel":0.0,"pitch":0.0,"pitch_rate":0.0,"roll":0.0,"roll_rate":0.0,"yaw":0.0,"yaw_rate":0.0,"lf_tire_info":null,"rf_tire_info":null,"lr_tire_info":null,"rr_tire_info":null,"annotations":[]}}}}"#
        ).unwrap();
        legacy_file.flush().unwrap();

        // Test that legacy format is detected
        assert!(is_legacy_format(&legacy_file.path().to_path_buf()));
    }

    #[test]
    fn test_new_format_not_detected_as_legacy() {
        // Create a temporary file with new format
        let mut new_file = NamedTempFile::new().unwrap();
        writeln!(
            new_file,
            r#"{{"DataPoint":{{"point_no":0,"timestamp_ms":1234567890,"game_source":"IRacing","gear":3,"speed_mps":50.0,"engine_rpm":5000.0,"max_engine_rpm":8000.0,"shift_point_rpm":7000.0,"throttle":0.8,"brake":0.0,"clutch":0.0,"steering":0.1,"steering_pct":0.05,"lap_distance":100.0,"lap_distance_pct":0.5,"lap_number":1,"last_lap_time_s":90.0,"best_lap_time_s":88.0,"is_pit_limiter_engaged":false,"is_in_pit_lane":false,"abs_active":false,"lat":0.0,"lon":0.0,"lat_accel":0.0,"lon_accel":0.0,"pitch":0.0,"pitch_rate":0.0,"roll":0.0,"roll_rate":0.0,"yaw":0.0,"yaw_rate":0.0,"lf_tire_info":null,"rf_tire_info":null,"lr_tire_info":null,"rr_tire_info":null,"annotations":[]}}}}"#
        ).unwrap();
        new_file.flush().unwrap();

        // Test that new format is NOT detected as legacy
        assert!(!is_legacy_format(&new_file.path().to_path_buf()));
    }

    #[test]
    fn test_load_legacy_format_returns_error() {
        // Create a temporary file with legacy format
        let mut legacy_file = NamedTempFile::new().unwrap();
        writeln!(
            legacy_file,
            r#"{{"DataPoint":{{"point_no":0,"point_epoch":1234567890,"lap_dist":100.0,"lap_dist_pct":0.5,"last_lap_time_s":90.0,"best_lap_time_s":88.0,"car_shift_ideal_rpm":7000.0,"cur_gear":3,"cur_rpm":5000.0,"cur_speed":50.0,"lap_no":1,"throttle":0.8,"brake":0.0,"steering":0.1,"steering_pct":0.05,"abs_active":false,"lat":0.0,"lon":0.0,"lat_accel":0.0,"lon_accel":0.0,"pitch":0.0,"pitch_rate":0.0,"roll":0.0,"roll_rate":0.0,"yaw":0.0,"yaw_rate":0.0,"lf_tire_info":null,"rf_tire_info":null,"lr_tire_info":null,"rr_tire_info":null,"annotations":[]}}}}"#
        ).unwrap();
        legacy_file.flush().unwrap();

        // Test that loading legacy format returns the correct error
        let result = load_telemetry_jsonl(&legacy_file.path().to_path_buf());
        assert!(result.is_err());
        match result {
            Err(OcypodeError::LegacyTelemetryFormat) => {
                // Expected error
            }
            _ => panic!("Expected LegacyTelemetryFormat error"),
        }
    }
}

fn load_telemetry_jsonl(source_file: &PathBuf) -> Result<TelemetryFile, OcypodeError> {
    // Check if this is a legacy format file before attempting to deserialize
    if is_legacy_format(source_file) {
        return Err(OcypodeError::LegacyTelemetryFormat);
    }
    
    // TODO: Should probably load in a non-blocking way here
    let telemetry_lines = serde_jsonlines::json_lines(source_file)
        .map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?
        .collect::<Result<Vec<TelemetryOutput>, std::io::Error>>()
        .map_err(|e| {
            // If deserialization fails, check if it might be a legacy format
            // that we didn't catch in the initial check
            if is_legacy_format(source_file) {
                OcypodeError::LegacyTelemetryFormat
            } else {
                OcypodeError::TelemetryLoaderError { source: e }
            }
        })?;

    let mut telemetry_data = TelemetryFile::default();
    let mut cur_lap_no: u32 = 0;
    let mut cur_session = Session::default();
    let mut cur_lap = Lap::default();
    for line in telemetry_lines {
        match line {
            TelemetryOutput::DataPoint(telemetry_point) => {
                let lap_no = telemetry_point.lap_number.unwrap_or(0);
                if lap_no != cur_lap_no {
                    cur_session.laps.push(cur_lap.clone());
                    cur_lap = Lap::default();
                    cur_lap_no = lap_no;
                }
                cur_lap.telemetry.push(telemetry_point);
            }
            TelemetryOutput::SessionChange(session_info) => {
                if !cur_lap.telemetry.is_empty() {
                    cur_session.laps.push(cur_lap);
                }
                // if we already have data points we are starting a new session
                if !cur_session.laps.is_empty() {
                    telemetry_data.sessions.push(cur_session.clone());
                    cur_session = Session::default();
                }
                cur_lap = Lap::default();
                cur_lap_no = 0;
                cur_session.info = session_info;
            }
        }
    }
    telemetry_data.sessions.push(cur_session);
    Ok(telemetry_data)
}
