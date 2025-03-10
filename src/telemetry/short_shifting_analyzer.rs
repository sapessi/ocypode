use std::default;

use super::{TelemetryAnalyzer, TelemetryAnnotation};

const DEFAULT_SHORT_SHIFT_SENSITIVITY: f32 = 100.;

pub(crate) struct ShortShiftingAnalyzer {
    prev_rpm: f32,
    prev_gear: u32,
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
        telemetry_point: &super::TelemetryPoint,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();
        if self.prev_rpm > 0.
            && self.prev_gear > 0
            && telemetry_point.cur_gear > self.prev_gear
            && self.prev_rpm < telemetry_point.car_shift_ideal_rpm - self.sensitivity
        {
            output.push(TelemetryAnnotation::ShortShifting {
                gear_change_rpm: self.prev_rpm,
                optimal_rpm: telemetry_point.car_shift_ideal_rpm,
                is_short_shifting: true,
            });
        }

        // skip double-clutching from short-shifting
        if telemetry_point.cur_gear > 0 {
            self.prev_gear = telemetry_point.cur_gear;
            self.prev_rpm = telemetry_point.cur_rpm;
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryAnnotation, TelemetryPoint};

    #[test]
    fn test_short_shift_annotation_inserted() {
        let mut analyzer = ShortShiftingAnalyzer::default();
        let mut telemetry_point = TelemetryPoint {
            cur_gear: 2,
            cur_rpm: 5000.0,
            car_shift_ideal_rpm: 6200.0,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        let mut output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
        telemetry_point.cur_gear = 3;
        telemetry_point.cur_rpm = 5100.;
        output = analyzer.analyze(&telemetry_point, &session_info);
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
        let mut telemetry_point = TelemetryPoint {
            cur_gear: 2,
            cur_rpm: 5100.0,
            car_shift_ideal_rpm: 5200.0,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        analyzer.analyze(&telemetry_point, &session_info);
        telemetry_point.cur_gear = 3;
        telemetry_point.cur_rpm = 5110.;
        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }
}
