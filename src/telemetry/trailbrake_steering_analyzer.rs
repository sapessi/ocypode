use std::collections::HashMap;

use super::{SessionInfo, TelemetryAnalyzer};

pub struct TrailbrakeSteeringAnalyzer {
    max_trailbraking_steering_angle: f32,
    min_trailbraking_pct: f32,
}

impl TrailbrakeSteeringAnalyzer {
    pub fn new(max_trailbraking_steering_angle: f32, min_trailbraking_pct: f32) -> Self {
        Self {
            max_trailbraking_steering_angle,
            min_trailbraking_pct,
        }
    }
}

impl TelemetryAnalyzer for TrailbrakeSteeringAnalyzer {
    fn analyze(
        &mut self,
        telemetry_point: &super::TelemetryPoint,
        session_info: &SessionInfo,
    ) -> std::collections::HashMap<String, super::TelemetryAnnotation> {
        let cur_steering_pct = telemetry_point.steering.abs() / session_info.max_steering_angle;
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

#[cfg(test)]
mod tests {
    use crate::telemetry::{TelemetryAnnotation, TelemetryPoint};

    use super::*;

    #[test]
    fn test_fires_annotations() {
        let mut analyzer = TrailbrakeSteeringAnalyzer::new(0.1, 0.2);
        let point = TelemetryPoint {
            brake: 0.3,
            steering: 0.2,
            ..Default::default()
        };
        let annotations = analyzer.analyze(&point, &SessionInfo::default());
        assert_eq!(annotations.len(), 2);
        assert_eq!(
            annotations.get("excessive_trailbrake_steering"),
            Some(&TelemetryAnnotation::Bool(true))
        );
    }
}
