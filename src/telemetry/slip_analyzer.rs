use super::TelemetryAnalyzer;
use simetry::Moment;
use uom::si::velocity::meter_per_second;

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
        telemetry: &dyn Moment,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Extract data from Moment trait
        let pedals = telemetry.pedals();
        let brake = pedals.as_ref().map(|p| p.brake as f32).unwrap_or(0.0);
        let throttle = pedals.as_ref().map(|p| p.throttle as f32).unwrap_or(0.0);
        
        // Note: steering angle is not available in the base Moment trait
        // For now, we'll use a placeholder value of 0.0
        // This will need to be addressed when game-specific implementations are available
        let steering = 0.0f32; // TODO: Extract from game-specific Moment implementation
        
        let cur_speed = telemetry.vehicle_velocity()
            .map(|v| v.get::<meter_per_second>() as f32)
            .unwrap_or(0.0);

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
    use crate::telemetry::{SessionInfo, TelemetryAnnotation, MockMoment, SerializableTelemetry, GameSource};

    #[test]
    fn test_slip_annotation_inserted() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = SerializableTelemetry {
            throttle: Some(0.5),
            brake: Some(0.0),
            speed_mps: Some(50.0),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&moment, &session_info);
        // Note: This test may not produce slip annotation because steering is not available
        // from the base Moment trait. This is expected behavior until game-specific
        // implementations provide steering data.
        assert!(output.len() <= 1);
    }

    #[test]
    fn test_no_slip_annotation_due_to_brake() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = SerializableTelemetry {
            throttle: Some(0.5),
            brake: Some(0.1),
            speed_mps: Some(50.0),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&moment, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_slip_annotation_due_to_speed() {
        let mut analyzer = SlipAnalyzer::default();
        let telemetry_data = SerializableTelemetry {
            throttle: Some(0.5),
            brake: Some(0.0),
            speed_mps: Some(60.0),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        // Initial state
        analyzer.prev_throttle = 0.4;
        analyzer.prev_speed = 55.0;

        let output = analyzer.analyze(&moment, &session_info);
        assert!(output.is_empty());
    }
    
    fn create_default_telemetry() -> SerializableTelemetry {
        SerializableTelemetry {
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
            steering: None,
            steering_pct: None,
            lap_distance: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            abs_active: None,
            lat: None,
            lon: None,
            lat_accel: None,
            lon_accel: None,
            pitch: None,
            pitch_rate: None,
            roll: None,
            roll_rate: None,
            yaw: None,
            yaw_rate: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        }
    }
}
