use super::{TelemetryAnalyzer, TelemetryData};

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
        telemetry: &TelemetryData,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Extract data from TelemetryData
        let brake = telemetry.brake.unwrap_or(0.0);
        let throttle = telemetry.throttle.unwrap_or(0.0);
        let steering = telemetry.steering_angle_rad.unwrap_or(0.0).abs();
        let cur_speed = telemetry.speed_mps.unwrap_or(0.0);

        if brake == 0.
            && throttle >= self.prev_throttle
            && steering > STEERING_ANGLE_DEADZONE_RAD
            && cur_speed < self.prev_speed
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
    use crate::telemetry::{GameSource, SessionInfo, TelemetryAnnotation, TelemetryData};

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
        // Should produce slip annotation: brake=0, throttle increasing, steering > deadzone, speed decreasing
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
            point_no: 0,
            timestamp_ms: 0,
            game_source: GameSource::IRacing,
            gear: Some(1),
            speed_mps: Some(0.0),
            engine_rpm: Some(0.0),
            max_engine_rpm: Some(6000.0),
            shift_point_rpm: Some(5500.0),
            throttle: Some(0.0),
            brake: Some(0.0),
            clutch: Some(0.0),
            steering_angle_rad: None,
            steering_pct: None,
            lap_distance_m: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            is_abs_active: None,
            latitude_deg: None,
            longitude_deg: None,
            lateral_accel_mps2: None,
            longitudinal_accel_mps2: None,
            pitch_rad: None,
            pitch_rate_rps: None,
            roll_rad: None,
            roll_rate_rps: None,
            yaw_rad: None,
            yaw_rate_rps: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        }
    }
}
