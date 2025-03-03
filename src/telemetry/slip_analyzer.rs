use super::TelemetryAnalyzer;

pub(crate) const STEERING_ANGLE_DEADZONE_RAD: f32 = 0.08;

#[derive(Default)]
pub(crate) struct SlipAnalyzer {
    prev_throttle: f32,
    prev_brake: f32,
    prev_steering_angle: f32,
    prev_speed: f32,
}

impl TelemetryAnalyzer for SlipAnalyzer {
    fn analyze(
        &mut self,
        telemetry_point: &super::TelemetryPoint,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        if telemetry_point.brake == 0.
            && telemetry_point.throttle >= self.prev_throttle
            && telemetry_point.steering > STEERING_ANGLE_DEADZONE_RAD
            && telemetry_point.cur_speed < self.prev_speed
        {
            output.push(super::TelemetryAnnotation::Slip {
                prev_speed: self.prev_speed,
                cur_speed: telemetry_point.cur_speed,
                is_slip: true,
            });
        }

        self.prev_throttle = telemetry_point.throttle;
        self.prev_brake = telemetry_point.brake;
        self.prev_steering_angle = telemetry_point.steering;
        self.prev_speed = telemetry_point.cur_speed;

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryAnnotation, TelemetryPoint};

    #[test]
    fn test_slip_annotation_inserted() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_point = TelemetryPoint {
            throttle: 0.5,
            brake: 0.0,
            steering: 0.1,
            cur_speed: 50.0,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert_eq!(output.len(), 1);
        assert!(match output.first().unwrap() {
            TelemetryAnnotation::Slip {
                prev_speed: _,
                cur_speed: _,
                is_slip,
            } => *is_slip,
            _ => false,
        });
    }

    #[test]
    fn test_no_slip_annotation_due_to_brake() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_point = TelemetryPoint {
            throttle: 0.5,
            brake: 0.1,
            steering: 0.1,
            cur_speed: 50.0,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_slip_annotation_due_to_steering() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_point = TelemetryPoint {
            throttle: 0.5,
            brake: 0.0,
            steering: 0.05,
            cur_speed: 50.0,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_slip_annotation_due_to_speed() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_point = TelemetryPoint {
            throttle: 0.5,
            brake: 0.0,
            steering: 0.1,
            cur_speed: 60.0,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }
}
