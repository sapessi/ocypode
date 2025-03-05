use std::time::SystemTime;

use egui::{Align, Color32, Image, ImageButton, Layout, Response, RichText, Ui};
use log::debug;

use crate::{
    telemetry::{TelemetryAnnotation, TelemetryPoint},
    OcypodeError,
};

pub(crate) mod analysis;
pub(crate) mod live;

const ALERT_DURATION_MS: u128 = 500;
pub(crate) type AlertImageSelector<'a> = fn(&TelemetryPoint) -> Image<'a>;

pub(crate) trait Alert {
    fn update_state(&mut self, telemetry_point: &TelemetryPoint) -> Result<(), OcypodeError>;
    fn show(&mut self, ui: &mut Ui, align: Align) -> Response;
}

pub(crate) struct DefaultAlert<'i> {
    image_selector: AlertImageSelector<'i>,
    current_image: Image<'i>,
    text: String,
    is_button: bool,
}

impl<'i> DefaultAlert<'i> {
    pub(crate) fn with_image(text: String, image_selector: AlertImageSelector<'i>) -> Self {
        Self {
            image_selector,
            text,
            current_image: image_selector(&TelemetryPoint::default()),
            is_button: false,
        }
    }

    pub(crate) fn abs() -> Self {
        Self::with_image("ABS".to_string(), |telemetry| {
            let mut abs_image = egui::include_image!("../../assets/brake-green.png");
            if telemetry.brake > 0.4 && !telemetry.abs_active {
                abs_image = egui::include_image!("../../assets/brake-orange.png");
            }
            if telemetry.abs_active {
                abs_image = egui::include_image!("../../assets/brake-red.png");
            }
            abs_image.into()
        })
    }

    pub(crate) fn shift() -> Self {
        Self::with_image("Shift".to_string(), |telemetry| {
            let mut shift_image = egui::include_image!("../../assets/shift-grey.png");
            if telemetry.cur_rpm > telemetry.car_shift_ideal_rpm - 100.
                && telemetry.cur_rpm < telemetry.car_shift_ideal_rpm + 100.
            {
                shift_image = egui::include_image!("../../assets/shift-green.png");
            }
            if telemetry.cur_rpm > telemetry.car_shift_ideal_rpm + 100. {
                shift_image = egui::include_image!("../../assets/shift-red.png");
            }

            telemetry.annotations.iter().find(|p| match p {
                TelemetryAnnotation::ShortShifting {
                    gear_change_rpm: _,
                    optimal_rpm: _,
                    is_short_shifting,
                } => {
                    if *is_short_shifting {
                        shift_image = egui::include_image!("../../assets/shift-orange.png");
                    }
                    true
                }
                _ => false,
            });

            shift_image.into()
        })
    }

    pub(crate) fn traction() -> Self {
        Self::with_image("Traction".to_string(), |telemetry| {
            let mut traction_image = egui::include_image!("../../assets/wheelspin-green.png");

            telemetry.annotations.iter().find(|p| match p {
                TelemetryAnnotation::Wheelspin {
                    avg_rpm_increase_per_gear: _,
                    cur_gear: _,
                    cur_rpm_increase: _,
                    is_wheelspin,
                } => {
                    if *is_wheelspin {
                        traction_image = egui::include_image!("../../assets/wheelspin-red.png");
                    }
                    true
                }
                _ => false,
            });

            traction_image.into()
        })
    }

    pub(crate) fn trailbrake_steering() -> Self {
        Self::with_image("Trailbraking".to_string(), |telemetry| {
            let mut trailbrake_image = egui::include_image!("../../assets/steering-grey.png");
            // trailbrake steering analyzer
            if telemetry.brake > 0.05 {
                trailbrake_image = egui::include_image!("../../assets/steering-green.png");
            }
            telemetry.annotations.iter().find(|p| match p {
                TelemetryAnnotation::TrailbrakeSteering {
                    cur_trailbrake_steering: _,
                    is_excessive_trailbrake_steering,
                } => {
                    if *is_excessive_trailbrake_steering {
                        trailbrake_image = egui::include_image!("../../assets/steering-red.png");
                    }
                    true
                }
                _ => false,
            });

            trailbrake_image.into()
        })
    }

    pub(crate) fn button(mut self) -> Self {
        self.is_button = true;
        self
    }
}

impl Alert for DefaultAlert<'_> {
    fn update_state(&mut self, telemetry_point: &TelemetryPoint) -> Result<(), OcypodeError> {
        self.current_image = (self.image_selector)(telemetry_point);
        Ok(())
    }

    fn show(&mut self, ui: &mut Ui, align: Align) -> Response {
        ui.with_layout(Layout::top_down(align), |ui| {
            ui.label(RichText::new(self.text.clone()).color(Color32::WHITE));
            if self.is_button {
                ui.add(ImageButton::new(self.current_image.clone()).frame(false))
            } else {
                ui.add(self.current_image.clone())
            }
        })
        .inner
    }
}

pub(crate) struct ScrubSlipAlert {
    alert_start_time: SystemTime,
    is_slip: bool,
    is_scrub: bool,
    is_button: bool,
}

impl Default for ScrubSlipAlert {
    fn default() -> Self {
        Self {
            alert_start_time: SystemTime::now(),
            is_slip: false,
            is_scrub: false,
            is_button: false,
        }
    }
}

impl ScrubSlipAlert {
    pub(crate) fn button(mut self) -> Self {
        self.is_button = true;
        self
    }
}

impl Alert for ScrubSlipAlert {
    fn update_state(&mut self, telemetry_point: &TelemetryPoint) -> Result<(), OcypodeError> {
        for annotation in &telemetry_point.annotations {
            match annotation {
                TelemetryAnnotation::Slip {
                    prev_speed: _,
                    cur_speed: _,
                    is_slip,
                } => {
                    self.is_slip = *is_slip;
                    self.alert_start_time = SystemTime::now();
                    break;
                }
                TelemetryAnnotation::Scrub {
                    avg_yaw_rate_change: _,
                    cur_yaw_rate_change: _,
                    is_scrubbing,
                } => {
                    self.is_scrub = *is_scrubbing;
                    self.alert_start_time = SystemTime::now();
                    break;
                }
                _ => continue,
            };
        }
        Ok(())
    }

    fn show(&mut self, ui: &mut Ui, button_align: Align) -> Response {
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
            if self.is_button {
                ui.add(ImageButton::new(turn_image).frame(false))
            } else {
                ui.add(Image::new(turn_image))
            }
        })
        .inner
    }
}

pub(crate) fn stroke_shade(start: Color32, end: Color32, y: f32) -> Color32 {
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
