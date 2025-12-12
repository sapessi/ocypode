use simple_moving_average::{SMA, SumTreeSMA};

use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

/// Minimum brake percentage to consider for entry oversteer detection
const MIN_BRAKE_PCT: f32 = 0.3;
/// Minimum steering percentage to consider for entry oversteer detection
const MIN_STEERING_PCT: f32 = 0.1;
/// Threshold multiplier for detecting oversteer (yaw rate exceeds expected by this factor)
const OVERSTEER_THRESHOLD: f32 = 1.5;

pub(crate) struct EntryOversteerAnalyzer<const WINDOW_SIZE: usize> {
    yaw_to_steering_window: SumTreeSMA<f32, f32, WINDOW_SIZE>,
    min_points: usize,
}

impl<const WINDOW_SIZE: usize> EntryOversteerAnalyzer<WINDOW_SIZE> {
    pub(crate) fn new(min_points: usize) -> Self {
        Self {
            yaw_to_steering_window: SumTreeSMA::new(),
            min_points,
        }
    }
}

impl<const WINDOW_SIZE: usize> TelemetryAnalyzer for EntryOversteerAnalyzer<WINDOW_SIZE> {
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
        let steering_pct = telemetry.steering_pct.unwrap_or(0.0);

        // Access yaw_rate_rps from TelemetryData, handle None gracefully
        let yaw_rate = match telemetry.yaw_rate_rps {
            Some(rate) => rate,
            None => {
                // If yaw rate is not available, we cannot perform entry oversteer analysis
                return output;
            }
        };

        // Only analyze during braking with steering input (corner entry phase)
        if brake > MIN_BRAKE_PCT && steering_pct.abs() > MIN_STEERING_PCT {
            // Calculate the ratio of yaw rate to steering input
            // This represents the expected yaw response for the given steering input
            let yaw_to_steering_ratio = yaw_rate.abs() / steering_pct.abs();

            // Once we have enough samples, check if current yaw rate exceeds expected
            // IMPORTANT: Check BEFORE adding the current sample to avoid polluting the baseline
            if self.yaw_to_steering_window.get_num_samples() >= self.min_points {
                let expected_ratio = self.yaw_to_steering_window.get_average();
                let expected_yaw_rate = steering_pct.abs() * expected_ratio;
                let actual_yaw_rate = yaw_rate.abs();

                // Detect oversteer: actual yaw rate significantly exceeds expected
                if actual_yaw_rate > expected_yaw_rate * OVERSTEER_THRESHOLD {
                    output.push(TelemetryAnnotation::EntryOversteer {
                        expected_yaw_rate,
                        actual_yaw_rate,
                        is_oversteer: true,
                    });
                }
            }

            // Add sample after detection to maintain clean baseline
            self.yaw_to_steering_window
                .add_sample(yaw_to_steering_ratio);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryData};
    use proptest::prelude::*;

    #[test]
    fn test_entry_oversteer_detected() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Build up baseline with normal yaw response
        for _ in 0..5 {
            let telemetry = TelemetryData {
                brake: Some(0.5),
                steering_pct: Some(0.3),
                yaw_rate_rps: Some(0.15), // Normal ratio: 0.15 / 0.3 = 0.5
                speed_mps: Some(10.),
                ..TelemetryData::default()
            };
            analyzer.analyze(&telemetry, &session_info);
        }

        // Now send telemetry with excessive yaw rate (oversteer)
        let telemetry = TelemetryData {
            brake: Some(0.5),
            steering_pct: Some(0.3),
            yaw_rate_rps: Some(0.3), // Excessive: 0.3 / 0.3 = 1.0, which is 2x the baseline
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::EntryOversteer {
                expected_yaw_rate,
                actual_yaw_rate,
                is_oversteer,
            } => {
                assert!(*is_oversteer);
                assert!(*actual_yaw_rate > *expected_yaw_rate);
            }
            _ => panic!("Expected EntryOversteer annotation"),
        }
    }

    #[test]
    fn test_no_oversteer_with_normal_yaw() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Build up baseline and continue with normal yaw response
        for _ in 0..10 {
            let telemetry = TelemetryData {
                brake: Some(0.5),
                steering_pct: Some(0.3),
                yaw_rate_rps: Some(0.15), // Normal ratio
                ..TelemetryData::default()
            };
            let output = analyzer.analyze(&telemetry, &session_info);
            // After the first 5 samples, we should not detect oversteer with normal yaw
            if analyzer.yaw_to_steering_window.get_num_samples() >= 5 {
                assert!(output.is_empty());
            }
        }
    }

    #[test]
    fn test_no_detection_without_braking() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            brake: Some(0.1), // Below threshold
            steering_pct: Some(0.3),
            yaw_rate_rps: Some(0.5), // High yaw rate
            ..TelemetryData::default()
        };

        for _ in 0..10 {
            let output = analyzer.analyze(&telemetry, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_no_detection_without_steering() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            brake: Some(0.5),
            steering_pct: Some(0.05), // Below threshold
            yaw_rate_rps: Some(0.5),  // High yaw rate
            ..TelemetryData::default()
        };

        for _ in 0..10 {
            let output = analyzer.analyze(&telemetry, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_no_detection_with_missing_yaw_rate() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            brake: Some(0.5),
            steering_pct: Some(0.3),
            yaw_rate_rps: None, // Missing yaw rate
            ..TelemetryData::default()
        };

        for _ in 0..10 {
            let output = analyzer.analyze(&telemetry, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_insufficient_samples() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Only provide 4 samples (less than min_points)
        for _ in 0..4 {
            let telemetry = TelemetryData {
                brake: Some(0.5),
                steering_pct: Some(0.3),
                yaw_rate_rps: Some(0.5), // High yaw rate
                ..TelemetryData::default()
            };
            let output = analyzer.analyze(&telemetry, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_state_reset_on_session_change() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Build up some state
        for _ in 0..5 {
            let telemetry = TelemetryData {
                brake: Some(0.5),
                steering_pct: Some(0.3),
                yaw_rate_rps: Some(0.15),
                speed_mps: Some(10.),
                ..TelemetryData::default()
            };
            analyzer.analyze(&telemetry, &session_info);
        }

        // Verify we have samples
        assert!(analyzer.yaw_to_steering_window.get_num_samples() >= 5);

        // Create a new analyzer (simulating session reset)
        let mut new_analyzer = EntryOversteerAnalyzer::<10>::new(5);

        // Verify new analyzer has no samples
        assert_eq!(new_analyzer.yaw_to_steering_window.get_num_samples(), 0);

        // Verify it doesn't detect anything without sufficient samples
        let telemetry = TelemetryData {
            brake: Some(0.5),
            steering_pct: Some(0.3),
            yaw_rate_rps: Some(0.5),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        let output = new_analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_pit_limiter_engaged() {
        let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Build up baseline with normal yaw response
        for _ in 0..5 {
            let telemetry = TelemetryData {
                brake: Some(0.5),
                steering_pct: Some(0.3),
                yaw_rate_rps: Some(0.15),
                is_pit_limiter_engaged: Some(false),
                ..TelemetryData::default()
            };
            analyzer.analyze(&telemetry, &session_info);
        }

        // Now send telemetry with excessive yaw rate but pit limiter engaged
        let telemetry = TelemetryData {
            brake: Some(0.5),
            steering_pct: Some(0.3),
            yaw_rate_rps: Some(0.3), // Would normally trigger oversteer
            is_pit_limiter_engaged: Some(true), // Pit limiter engaged
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        // Should not detect anything when pit limiter is engaged
        assert!(output.is_empty());
    }

    // **Feature: setup-assistant, Property 12: Entry oversteer detection**
    // **Validates: Requirements 11.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_entry_oversteer_detection(
            baseline_brake in MIN_BRAKE_PCT + 0.01f32..1.0f32,
            baseline_steering in MIN_STEERING_PCT + 0.01f32..1.0f32,
            baseline_yaw_ratio in 0.3f32..0.7f32,
            oversteer_multiplier in OVERSTEER_THRESHOLD + 0.1f32..3.0f32,
        ) {
            let mut analyzer = EntryOversteerAnalyzer::<10>::new(5);
            let session_info = SessionInfo::default();

            // Build up baseline with normal yaw response
            let baseline_yaw = baseline_steering * baseline_yaw_ratio;
            for _ in 0..5 {
                let telemetry = TelemetryData {
                    brake: Some(baseline_brake),
                    steering_pct: Some(baseline_steering),
                    yaw_rate_rps: Some(baseline_yaw),
                    speed_mps: Some(10.),
                    ..TelemetryData::default()
                };
                analyzer.analyze(&telemetry, &session_info);
            }

            // Now send telemetry with excessive yaw rate (oversteer)
            let excessive_yaw = baseline_steering * baseline_yaw_ratio * oversteer_multiplier;
            let telemetry = TelemetryData {
                brake: Some(baseline_brake),
                steering_pct: Some(baseline_steering),
                yaw_rate_rps: Some(excessive_yaw),
                speed_mps: Some(10.),
                ..TelemetryData::default()
            };

            let output = analyzer.analyze(&telemetry, &session_info);

            // Property: For any telemetry with brake application, steering input,
            // and yaw rate exceeding expected response by OVERSTEER_THRESHOLD,
            // the analyzer should detect oversteer
            prop_assert_eq!(output.len(), 1);
            match &output[0] {
                TelemetryAnnotation::EntryOversteer {
                    expected_yaw_rate,
                    actual_yaw_rate,
                    is_oversteer,
                } => {
                    prop_assert!(*is_oversteer);
                    prop_assert!(*actual_yaw_rate > *expected_yaw_rate);
                }
                _ => prop_assert!(false, "Expected EntryOversteer annotation"),
            }
        }
    }
}
