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
            && shift_point_rpm > 0.0
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

    #[test]
    fn test_acc_short_shift_annotation_with_estimated_shift_point() {
        let mut analyzer = ShortShiftingAnalyzer::default();

        // Create ACC telemetry data with estimated shift_point_rpm (now populated by from_acc_state)
        let telemetry_data = TelemetryData {
            game_source: GameSource::ACC,
            gear: Some(2),
            engine_rpm: Some(5000.0),
            max_engine_rpm: Some(7300.0),
            shift_point_rpm: Some(6351.0), // Now populated: 7300 * 0.87 = 6351
            speed_mps: Some(10.),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        let mut output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());

        // Shift up at low RPM (should trigger short shift detection)
        // Shifting at 5100 RPM is well below optimal (6351 - 100 = 6251)
        let telemetry_data2 = TelemetryData {
            game_source: GameSource::ACC,
            gear: Some(3),
            engine_rpm: Some(5100.0),
            max_engine_rpm: Some(7300.0),
            shift_point_rpm: Some(6351.0),
            speed_mps: Some(10.),
            ..create_default_telemetry()
        };
        output = analyzer.analyze(&telemetry_data2, &session_info);

        assert_eq!(output.len(), 1);
        assert!(match output.first().unwrap() {
            TelemetryAnnotation::ShortShifting {
                gear_change_rpm,
                optimal_rpm,
                is_short_shifting,
            } => {
                *is_short_shifting && *gear_change_rpm == 5000.0 && *optimal_rpm == 6351.0
            }
            _ => false,
        });
    }

    #[test]
    fn test_acc_no_short_shift_annotation_when_shifting_near_optimal() {
        let mut analyzer = ShortShiftingAnalyzer::default();

        // Create ACC telemetry data with estimated shift point
        let telemetry_data = TelemetryData {
            game_source: GameSource::ACC,
            gear: Some(2),
            engine_rpm: Some(6300.0), // Close to optimal shift point
            max_engine_rpm: Some(7300.0),
            shift_point_rpm: Some(6351.0), // 7300 * 0.87 = 6351
            speed_mps: Some(10.),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo::default();

        analyzer.analyze(&telemetry_data, &session_info);

        // Shift at near-optimal RPM (should not trigger short shift)
        // Optimal = 6351, shifting at 6300 is within sensitivity (100 RPM)
        let telemetry_data2 = TelemetryData {
            game_source: GameSource::ACC,
            gear: Some(3),
            engine_rpm: Some(6310.0),
            max_engine_rpm: Some(7300.0),
            shift_point_rpm: Some(6351.0),
            speed_mps: Some(10.),
            ..create_default_telemetry()
        };
        let output = analyzer.analyze(&telemetry_data2, &session_info);
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
