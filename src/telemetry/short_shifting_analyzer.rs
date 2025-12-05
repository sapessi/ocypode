use std::default;
use simetry::Moment;
use uom::si::angular_velocity::revolution_per_minute;

use super::{TelemetryAnalyzer, TelemetryAnnotation};

const DEFAULT_SHORT_SHIFT_SENSITIVITY: f32 = 100.;

pub(crate) struct ShortShiftingAnalyzer {
    prev_rpm: f32,
    prev_gear: i8,
    sensitivity: f32,
}

impl default::Default for ShortShiftingAnalyzer {
    fn default() -> Self {
        Self {
            prev_gear: 0,
            prev_rpm: 0.,
            sensitivity: DEFAULT_SHORT_SHIFT_SENSITIVITY,
        }
    }
}

impl TelemetryAnalyzer for ShortShiftingAnalyzer {
    fn analyze(
        &mut self,
        telemetry: &dyn Moment,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();
        
        // Extract data from Moment trait
        let cur_gear = telemetry.vehicle_gear().unwrap_or(0);
        let cur_rpm = telemetry.vehicle_engine_rotation_speed()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32)
            .unwrap_or(0.0);
        let shift_point_rpm = telemetry.shift_point()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32)
            .unwrap_or(0.0);
        
        if self.prev_rpm > 0.
            && self.prev_gear > 0
            && cur_gear > self.prev_gear
            && self.prev_rpm < shift_point_rpm - self.sensitivity
        {
            output.push(TelemetryAnnotation::ShortShifting {
                gear_change_rpm: self.prev_rpm,
                optimal_rpm: shift_point_rpm,
                is_short_shifting: true,
            });
        }

        // skip double-clutching from short-shifting
        if cur_gear > 0 {
            self.prev_gear = cur_gear;
            self.prev_rpm = cur_rpm;
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryAnnotation, MockMoment, SerializableTelemetry, GameSource};

    #[test]
    fn test_short_shift_annotation_inserted() {
        let mut analyzer = ShortShiftingAnalyzer::default();
        let telemetry_data = SerializableTelemetry {
            gear: Some(2),
            engine_rpm: Some(5000.0),
            shift_point_rpm: Some(6200.0),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        let mut output = analyzer.analyze(&moment, &session_info);
        assert!(output.is_empty());
        
        let telemetry_data2 = SerializableTelemetry {
            gear: Some(3),
            engine_rpm: Some(5100.0),
            shift_point_rpm: Some(6200.0),
            ..create_default_telemetry()
        };
        let moment2 = MockMoment::new(telemetry_data2);
        output = analyzer.analyze(&moment2, &session_info);
        assert_eq!(output.len(), 1);
        assert!(match output.first().unwrap() {
            TelemetryAnnotation::ShortShifting {
                gear_change_rpm: _,
                optimal_rpm: _,
                is_short_shifting,
            } => *is_short_shifting,
            _ => false,
        });
    }

    #[test]
    fn test_no_short_shift_annotation() {
        let mut analyzer = ShortShiftingAnalyzer::default();
        let telemetry_data = SerializableTelemetry {
            gear: Some(2),
            engine_rpm: Some(5100.0),
            shift_point_rpm: Some(5200.0),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        analyzer.analyze(&moment, &session_info);
        
        let telemetry_data2 = SerializableTelemetry {
            gear: Some(3),
            engine_rpm: Some(5110.0),
            shift_point_rpm: Some(5200.0),
            ..create_default_telemetry()
        };
        let moment2 = MockMoment::new(telemetry_data2);
        let output = analyzer.analyze(&moment2, &session_info);
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
