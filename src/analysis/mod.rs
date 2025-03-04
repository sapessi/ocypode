use std::{path::PathBuf, sync::Arc};

use egui::{
    style::Widgets, Color32, Direction, Frame, Layout, Margin, RichText, TextEdit, Ui, Vec2b,
    Visuals,
};
use egui_dropdown::DropDownBox;
use egui_plot::{Legend, Line, PlotPoints, Points};
use itertools::Itertools;

use crate::{
    live::{
        telemetry_view::stroke_shade, PALETTE_BLACK, PALETTE_BROWN, PALETTE_MAROON, PALETTE_ORANGE,
    },
    telemetry::{SessionInfo, TelemetryOutput, TelemetryPoint},
    OcypodeError,
};

#[derive(Default, Clone, Debug)]
struct TelemetryFile {
    sessions: Vec<Session>,
}

#[derive(Default, Clone, Debug)]
struct Lap {
    telemetry: Vec<TelemetryPoint>,
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
            ui.separator();
            ui.label(RichText::new("Lap: ").color(Color32::WHITE));
            if let Some(selected_session) = self
                .data
                .as_ref()
                .unwrap()
                .sessions
                .iter()
                .find(|p| p.info.track_name == self.selected_session)
            {
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
                    throttle_vec.push([p.0 as f64, p.1.throttle as f64 * 100.]);
                    brake_vec.push([p.0 as f64, p.1.brake as f64 * 100.]);
                    steering_vec.push([p.0 as f64, 50. + 50. * p.1.steering_pct as f64]);
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
                    });
                if plot_response.response.clicked() {
                    if let Some(mouse_pos) = plot_response.response.interact_pointer_pos() {
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
        let cur_ui_stae = self.ui_state.clone();
        match cur_ui_stae {
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
                    .resizable(true)
                    .max_width(ctx.available_rect().width() * 0.3)
                    .show(ctx, |local_ui| {
                        if let Ok(selected_lap) = self.selected_lap.parse::<usize>() {
                            if let Some(x_point) = self.selected_x {
                                if let Some(lap) = session.laps.get(selected_lap) {
                                    if let Some(telemetry) = lap.telemetry.get(x_point) {
                                        let mut str_buffer =
                                            serde_json::to_string_pretty(&telemetry.annotations)
                                                .unwrap();
                                        local_ui.add(
                                            TextEdit::multiline(&mut str_buffer)
                                                .interactive(false)
                                                .desired_width(ctx.available_rect().width() * 0.3)
                                                .code_editor(),
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

fn load_telemetry_jsonl(source_file: &PathBuf) -> Result<TelemetryFile, OcypodeError> {
    // TODO: Shoudl probably load in a non-blocking way here
    let telemetry_lines = serde_jsonlines::json_lines(source_file)
        .map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?
        .collect::<Result<Vec<TelemetryOutput>, std::io::Error>>()
        .map_err(|e| OcypodeError::TelemetryLoaderError { source: e })?;

    let mut telemetry_data = TelemetryFile::default();
    let mut cur_lap_no: u32 = 0;
    let mut cur_session = Session::default();
    let mut cur_lap = Lap::default();
    for line in telemetry_lines {
        match line {
            TelemetryOutput::DataPoint(telemetry_point) => {
                if telemetry_point.lap_no != cur_lap_no {
                    cur_session.laps.push(cur_lap.clone());
                    cur_lap = Lap::default();
                    cur_lap_no = telemetry_point.lap_no;
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
