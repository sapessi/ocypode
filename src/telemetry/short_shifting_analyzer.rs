use std::default;

use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

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
        telemetry: &TelemetryData,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Skip analysis if doesn't meet requirements
        if !is_telemetry_point_analyzable(telemetry) {
            return output;
        }

        // Extract data from TelemetryData
        let cur_gear = telemetry.gear.unwrap_or(0);
        let cur_rpm = telemetry.engine_rpm.unwrap_or(0.0);
        let shift_point_rpm = telemetry.shift_point_rpm.unwrap_or(0.0);

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
    use crate::telemetry::{GameSource, SessionInfo, TelemetryAnnotation, TelemetryData};

    #[test]
    fn test_short_shift_annotation_inserted() {
        let mut analyzer = ShortShiftingAnalyzer::default();
        let telemetry_data = TelemetryData {
            gear: Some(2),
            engine_rpm: Some(5000.0),
            shift_point_rpm: Some(6200.0),
            speed_mps: Some(10.),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        let mut output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());

        let telemetry_data2 = TelemetryData {
            gear: Some(3),
            engine_rpm: Some(5100.0),
            shift_point_rpm: Some(6200.0),
            speed_mps: Some(10.),
            ..create_default_telemetry()
        };
        output = analyzer.analyze(&telemetry_data2, &session_info);
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
        let telemetry_data = TelemetryData {
            gear: Some(2),
            engine_rpm: Some(5100.0),
            shift_point_rpm: Some(5200.0),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        analyzer.analyze(&telemetry_data, &session_info);

        let telemetry_data2 = TelemetryData {
            gear: Some(3),
            engine_rpm: Some(5110.0),
            shift_point_rpm: Some(5200.0),
            ..create_default_telemetry()
        };
        let output = analyzer.analyze(&telemetry_data2, &session_info);
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
