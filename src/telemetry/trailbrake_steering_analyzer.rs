use std::collections::HashMap;

use super::{SessionInfo, TelemetryAnalyzer};

pub(crate) const MIN_TRAILBRAKING_PCT: f32 = 0.2;
pub(crate) const MAX_TRAILBRAKING_STEERING_ANGLE: f32 = 0.1;
pub(crate) const TRAILBRAKE_EXCESSIVE_STEERING_ANNOTATION: &str = "excessive_trailbrake_steering";
pub(crate) const TRAILBRAKE_STEERING_PCT_ANNOTATION: &str = "steering_pct";

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
        // nothing to process here if we cannot establish the current steering pct
        if session_info.max_steering_angle == 0. {
            return HashMap::new();
        }
        // this should not be possible
        if telemetry_point.steering > session_info.max_steering_angle {
            return HashMap::new();
        }

        let cur_steering_pct =
            telemetry_point.steering.abs() / session_info.max_steering_angle.abs();
        // we are braking... measure steering angle
        let mut annotations = HashMap::new();
        if telemetry_point.brake > self.min_trailbraking_pct
            && cur_steering_pct > self.max_trailbraking_steering_angle
        {
            annotations.insert(
                TRAILBRAKE_EXCESSIVE_STEERING_ANNOTATION.to_string(),
                super::TelemetryAnnotation::Bool(true),
            );
            annotations.insert(
                TRAILBRAKE_STEERING_PCT_ANNOTATION.to_string(),
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

    fn default_analyzer() -> TrailbrakeSteeringAnalyzer {
        TrailbrakeSteeringAnalyzer::new(0.1, 0.2)
    }

    #[test]
    fn test_fails_if_no_max_steering() {
        let mut analyzer = default_analyzer();
        let point = TelemetryPoint::default();
        let annotations = analyzer.analyze(&point, &SessionInfo::default());
        assert!(annotations.is_empty());
    }

    #[test]
    fn test_fires_annotations() {
        let mut analyzer = default_analyzer();
        let point = TelemetryPoint {
            brake: 0.3,
            steering: 0.2,
            ..Default::default()
        };
        let session_info = SessionInfo {
            max_steering_angle: 0.5,
            ..Default::default()
        };
        let annotations = analyzer.analyze(&point, &session_info);
        assert_eq!(annotations.len(), 2);
        assert_eq!(
            annotations.get(TRAILBRAKE_EXCESSIVE_STEERING_ANNOTATION),
            Some(&TelemetryAnnotation::Bool(true))
        );
        assert_eq!(
            annotations.get(TRAILBRAKE_STEERING_PCT_ANNOTATION),
            Some(&TelemetryAnnotation::Float(
                point.steering.abs() / session_info.max_steering_angle
            ))
        );
    }

    #[test]
    fn test_doesnt_annotations() {
        let mut analyzer = default_analyzer();
        let mut point = TelemetryPoint {
            brake: 0.1,
            steering: 0.2,
            ..Default::default()
        };
        let mut session_info = SessionInfo {
            max_steering_angle: 0.5,
            ..Default::default()
        };
        assert!(analyzer.analyze(&point, &session_info).is_empty());

        point.brake = 0.3;
        point.steering = 0.001;
        assert!(analyzer.analyze(&point, &session_info).is_empty());

        point.brake = 0.001;
        assert!(analyzer.analyze(&point, &session_info).is_empty());

        point.brake = 0.3;
        point.steering = 2.;
        session_info.max_steering_angle = 1.;
        assert!(analyzer.analyze(&point, &session_info).is_empty());
    }
}
