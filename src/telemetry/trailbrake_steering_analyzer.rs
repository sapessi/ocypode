use super::{SessionInfo, TelemetryAnalyzer};

pub(crate) const MIN_TRAILBRAKING_PCT: f32 = 0.2;
pub(crate) const MAX_TRAILBRAKING_STEERING_ANGLE: f32 = 0.1;

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
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();
        // nothing to process here if we cannot establish the current steering pct
        if session_info.max_steering_angle == 0. {
            return output;
        }
        // this should not be possible
        if telemetry_point.steering > session_info.max_steering_angle {
            return output;
        }

        // we are braking... measure steering angle
        if telemetry_point.brake > self.min_trailbraking_pct
            && telemetry_point.steering_pct > self.max_trailbraking_steering_angle
        {
            output.push(super::TelemetryAnnotation::TrailbrakeSteering {
                cur_trailbrake_steering: telemetry_point.steering_pct,
                is_excessive_trailbrake_steering: true,
            });
        }
        output
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
            steering_pct: 0.3,
            ..Default::default()
        };
        let session_info = SessionInfo {
            max_steering_angle: 0.5,
            ..Default::default()
        };
        let annotations = analyzer.analyze(&point, &session_info);
        assert_eq!(annotations.len(), 1);
        assert_eq!(
            *annotations.get(0).unwrap(),
            TelemetryAnnotation::TrailbrakeSteering {
                cur_trailbrake_steering: 0.3,
                is_excessive_trailbrake_steering: true
            }
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
