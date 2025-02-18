use std::collections::HashMap;

use iracing::telemetry::Sample;

use crate::telemetry::collector::get_float;

use super::TelemetryAnalyzer;

pub struct TrailbrakeSteeringAnalyzer {
    max_trailbraking_steering_angle: f32,
    min_trailbraking_pct: f32,
    max_steering_angle: Option<f32>,
}

impl TrailbrakeSteeringAnalyzer {
    pub fn new(max_trailbraking_steering_angle: f32, min_trailbraking_pct: f32) -> Self {
        Self {
            max_trailbraking_steering_angle,
            min_trailbraking_pct,
            max_steering_angle: None,
        }
    }
}

impl TelemetryAnalyzer for TrailbrakeSteeringAnalyzer {
    fn analyze(
        &mut self,
        telemetry_point: &super::TelemetryPoint,
        sample: &Sample,
    ) -> std::collections::HashMap<String, super::TelemetryAnnotation> {
        if self.max_steering_angle.is_none() {
            let max_angle = get_float(sample, "SteeringWheelAngleMax");
            if max_angle > 0.0 {
                self.max_steering_angle = Some(max_angle);
            } else {
                return HashMap::new();
            }
        }
        let cur_steering_pct = telemetry_point.steering.abs()
            / self
                .max_steering_angle
                .expect("Max steering angle not populated");
        // we are braking... measure steering angle
        let mut annotations = HashMap::new();
        if telemetry_point.brake > self.min_trailbraking_pct
            && cur_steering_pct > self.max_trailbraking_steering_angle
        {
            annotations.insert(
                "excessive_trailbrake_steering".to_string(),
                super::TelemetryAnnotation::Bool(true),
            );
            annotations.insert(
                "steering_pct".to_string(),
                super::TelemetryAnnotation::Float(cur_steering_pct),
            );
        }
        annotations
    }
}
