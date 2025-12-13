use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryData};

pub(crate) const STEERING_ANGLE_DEADZONE_RAD: f32 = 0.12; // Increased from 0.08 to reduce sensitivity

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
        telemetry: &TelemetryData,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Skip analysis if doesn't meet requirements
        if !is_telemetry_point_analyzable(telemetry) {
            return output;
        }

        // Extract data from TelemetryData
        let brake = telemetry.brake.unwrap_or(0.0);
        let throttle = telemetry.throttle.unwrap_or(0.0);
        let steering = telemetry.steering_angle_rad.unwrap_or(0.0).abs();
        let cur_speed = telemetry.speed_mps.unwrap_or(0.0);

        // Require more significant speed loss to reduce false positives
        const MIN_SPEED_LOSS_MPS: f32 = 0.5; // ~1.8 km/h minimum speed loss

        if brake == 0.
            && throttle >= self.prev_throttle
            && steering > STEERING_ANGLE_DEADZONE_RAD
            && cur_speed < self.prev_speed
            && (self.prev_speed - cur_speed) >= MIN_SPEED_LOSS_MPS
        {
            output.push(super::TelemetryAnnotation::Slip {
                prev_speed: self.prev_speed,
                cur_speed,
                is_slip: true,
            });
        }

        self.prev_throttle = throttle;
        self.prev_brake = brake;
        self.prev_steering_angle = steering;
        self.prev_speed = cur_speed;

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryAnnotation, TelemetryData};

    #[test]
    fn test_slip_annotation_inserted() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = TelemetryData {
            throttle: Some(0.5),
            brake: Some(0.0),
            speed_mps: Some(50.0),
            steering_angle_rad: Some(0.15), // Above deadzone
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_data, &session_info);
        // Should produce slip annotation: brake=0, throttle increasing, steering > deadzone, speed decreasing by >0.5 m/s
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::Slip {
                prev_speed,
                cur_speed,
                is_slip,
            } => {
                assert_eq!(*prev_speed, 55.0);
                assert_eq!(*cur_speed, 50.0);
                assert!(is_slip);
            }
            _ => panic!("Expected Slip annotation"),
        }
    }

    #[test]
    fn test_no_slip_annotation_due_to_brake() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = TelemetryData {
            throttle: Some(0.5),
            brake: Some(0.1),
            speed_mps: Some(50.0),
            steering_angle_rad: Some(0.15),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_slip_annotation_due_to_speed() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = TelemetryData {
            throttle: Some(0.5),
            brake: Some(0.0),
            speed_mps: Some(60.0),
            steering_angle_rad: Some(0.15),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_slip_annotation_due_to_insufficient_speed_loss() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = TelemetryData {
            throttle: Some(0.5),
            brake: Some(0.0),
            speed_mps: Some(54.8), // Only 0.2 m/s loss, below 0.5 threshold
            steering_angle_rad: Some(0.15),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_slip_annotation_due_to_low_steering() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = TelemetryData {
            throttle: Some(0.5),
            brake: Some(0.0),
            speed_mps: Some(50.0),
            steering_angle_rad: Some(0.05), // Below deadzone
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());
    }

    fn create_default_telemetry() -> TelemetryData {
        TelemetryData {
            gear: Some(1),
            speed_mps: Some(0.0),
            engine_rpm: Some(0.0),
            max_engine_rpm: Some(6000.0),
            shift_point_rpm: Some(5500.0),
            throttle: Some(0.0),
            brake: Some(0.0),
            clutch: Some(0.0),
            ..Default::default()
        }
    }
}
