use simple_moving_average::{SMA, SumTreeSMA};

use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

/// Maximum throttle percentage to consider for mid-corner coasting detection
const MAX_COASTING_THROTTLE: f32 = 0.15;
/// Maximum brake percentage to consider for mid-corner coasting detection
const MAX_COASTING_BRAKE: f32 = 0.15;
/// Minimum steering percentage to consider for mid-corner detection
const MIN_STEERING_PCT: f32 = 0.1;
/// Threshold for detecting understeer via speed loss (m/s)
const UNDERSTEER_SPEED_LOSS_THRESHOLD: f32 = 0.5;
/// Threshold multiplier for detecting oversteer (yaw rate exceeds expected by this factor)
const OVERSTEER_THRESHOLD: f32 = 1.5;

pub(crate) struct MidCornerAnalyzer<const WINDOW_SIZE: usize> {
    prev_speed: f32,
    yaw_to_steering_baseline: SumTreeSMA<f32, f32, WINDOW_SIZE>,
    min_points: usize,
}

impl<const WINDOW_SIZE: usize> MidCornerAnalyzer<WINDOW_SIZE> {
    pub(crate) fn new(min_points: usize) -> Self {
        Self {
            prev_speed: 0.0,
            yaw_to_steering_baseline: SumTreeSMA::new(),
            min_points,
        }
    }
}

impl<const WINDOW_SIZE: usize> TelemetryAnalyzer for MidCornerAnalyzer<WINDOW_SIZE> {
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
        let steering_pct = telemetry.steering_pct.unwrap_or(0.0);
        let cur_speed = telemetry.speed_mps.unwrap_or(0.0);

        // Access yaw_rate_rps from TelemetryData, handle None gracefully
        let yaw_rate = match telemetry.yaw_rate_rps {
            Some(rate) => rate,
            None => {
                // Update previous speed for next iteration
                self.prev_speed = cur_speed;
                return output;
            }
        };

        // Only analyze during mid-corner coasting phase (minimal throttle/brake with steering)
        if throttle < MAX_COASTING_THROTTLE
            && brake < MAX_COASTING_BRAKE
            && steering_pct.abs() > MIN_STEERING_PCT
        {
            // Detect understeer: speed loss while steering applied
            if self.prev_speed > 0.0 {
                let speed_loss = self.prev_speed - cur_speed;
                if speed_loss > UNDERSTEER_SPEED_LOSS_THRESHOLD {
                    output.push(TelemetryAnnotation::MidCornerUndersteer {
                        speed_loss,
                        is_understeer: true,
                    });
                }
            }

            // Detect oversteer: excessive yaw rate compared to steering input
            // Calculate the ratio of yaw rate to steering input
            let yaw_to_steering_ratio = yaw_rate.abs() / steering_pct.abs();

            // Once we have enough samples, check if current yaw rate exceeds expected
            if self.yaw_to_steering_baseline.get_num_samples() >= self.min_points {
                let expected_ratio = self.yaw_to_steering_baseline.get_average();
                let expected_yaw_rate = steering_pct.abs() * expected_ratio;
                let actual_yaw_rate = yaw_rate.abs();

                // Detect oversteer: actual yaw rate significantly exceeds expected
                if actual_yaw_rate > expected_yaw_rate * OVERSTEER_THRESHOLD {
                    let yaw_rate_excess = actual_yaw_rate - expected_yaw_rate;
                    output.push(TelemetryAnnotation::MidCornerOversteer {
                        yaw_rate_excess,
                        is_oversteer: true,
                    });
                }
            }

            // Add sample after detection to maintain clean baseline
            self.yaw_to_steering_baseline
                .add_sample(yaw_to_steering_ratio);
        }

        // Update previous speed for next iteration
        self.prev_speed = cur_speed;

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryData};
    use proptest::prelude::*;

    #[test]
    fn test_mid_corner_understeer_detected() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Set initial speed
        analyzer.prev_speed = 50.0;

        // Send telemetry with speed loss during coasting with steering
        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.1),
            steering_pct: Some(0.3),
            speed_mps: Some(48.0), // Speed loss of 2.0 m/s
            yaw_rate_rps: Some(0.15),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::MidCornerUndersteer {
                speed_loss,
                is_understeer,
            } => {
                assert!(*is_understeer);
                assert!(*speed_loss > UNDERSTEER_SPEED_LOSS_THRESHOLD);
            }
            _ => panic!("Expected MidCornerUndersteer annotation"),
        }
    }

    #[test]
    fn test_mid_corner_oversteer_detected() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Build up baseline with normal yaw response
        for _ in 0..5 {
            let telemetry = TelemetryData {
                throttle: Some(0.1),
                brake: Some(0.1),
                steering_pct: Some(0.3),
                speed_mps: Some(50.0),
                yaw_rate_rps: Some(0.15), // Normal ratio: 0.15 / 0.3 = 0.5
                ..TelemetryData::default()
            };
            analyzer.analyze(&telemetry, &session_info);
        }

        // Now send telemetry with excessive yaw rate (oversteer)
        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.1),
            steering_pct: Some(0.3),
            speed_mps: Some(50.0),
            yaw_rate_rps: Some(0.3), // Excessive: 0.3 / 0.3 = 1.0, which is 2x the baseline
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::MidCornerOversteer {
                yaw_rate_excess,
                is_oversteer,
            } => {
                assert!(*is_oversteer);
                assert!(*yaw_rate_excess > 0.0);
            }
            _ => panic!("Expected MidCornerOversteer annotation"),
        }
    }

    #[test]
    fn test_no_detection_with_throttle() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        analyzer.prev_speed = 50.0;

        let telemetry = TelemetryData {
            throttle: Some(0.5), // Above threshold
            brake: Some(0.1),
            steering_pct: Some(0.3),
            speed_mps: Some(48.0),
            yaw_rate_rps: Some(0.5),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_brake() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        analyzer.prev_speed = 50.0;

        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.5), // Above threshold
            steering_pct: Some(0.3),
            speed_mps: Some(48.0),
            yaw_rate_rps: Some(0.5),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_without_steering() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        analyzer.prev_speed = 50.0;

        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.1),
            steering_pct: Some(0.05), // Below threshold
            speed_mps: Some(48.0),
            yaw_rate_rps: Some(0.5),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_missing_yaw_rate() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        analyzer.prev_speed = 50.0;

        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.1),
            steering_pct: Some(0.3),
            speed_mps: Some(48.0),
            yaw_rate_rps: None, // Missing yaw rate
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_understeer_with_small_speed_loss() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        analyzer.prev_speed = 50.0;

        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.1),
            steering_pct: Some(0.3),
            speed_mps: Some(49.8), // Speed loss of 0.2 m/s (below threshold)
            yaw_rate_rps: Some(0.15),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_insufficient_samples_for_oversteer() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Only provide 4 samples (less than min_points)
        for _ in 0..4 {
            let telemetry = TelemetryData {
                throttle: Some(0.1),
                brake: Some(0.1),
                steering_pct: Some(0.3),
                speed_mps: Some(50.0),
                yaw_rate_rps: Some(0.5), // High yaw rate
                ..TelemetryData::default()
            };
            let output = analyzer.analyze(&telemetry, &session_info);
            // Should not detect oversteer without sufficient samples
            assert!(
                output.is_empty()
                    || matches!(output[0], TelemetryAnnotation::MidCornerUndersteer { .. })
            );
        }
    }

    #[test]
    fn test_no_detection_with_pit_limiter_engaged() {
        let mut analyzer = MidCornerAnalyzer::<10>::new(5);
        let session_info = SessionInfo::default();

        // Set initial speed
        analyzer.prev_speed = 50.0;

        // Send telemetry with conditions that would trigger understeer, but pit limiter engaged
        let telemetry = TelemetryData {
            throttle: Some(0.1),
            brake: Some(0.1),
            steering_pct: Some(0.3),
            speed_mps: Some(48.0), // Speed loss that would trigger understeer
            yaw_rate_rps: Some(0.15),
            is_pit_limiter_engaged: Some(true), // Pit limiter engaged
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        // Should not detect anything when pit limiter is engaged
        assert!(output.is_empty());
    }

    // **Feature: setup-assistant, Property 14: Mid-corner understeer detection**
    // **Validates: Requirements 12.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_mid_corner_understeer_detection(
            throttle in 0.0f32..MAX_COASTING_THROTTLE,
            brake in 0.0f32..MAX_COASTING_BRAKE,
            steering in MIN_STEERING_PCT + 0.01f32..1.0f32,
            initial_speed in 20.0f32..100.0f32,
            speed_loss in UNDERSTEER_SPEED_LOSS_THRESHOLD + 0.1f32..10.0f32,
        ) {
            let mut analyzer = MidCornerAnalyzer::<10>::new(5);
            let session_info = SessionInfo::default();

            // Set initial speed
            analyzer.prev_speed = initial_speed;

            // Send telemetry with speed loss during coasting with steering
            let telemetry = TelemetryData {
                throttle: Some(throttle),
                brake: Some(brake),
                steering_pct: Some(steering),
                speed_mps: Some(initial_speed - speed_loss),
                yaw_rate_rps: Some(0.15),
                ..TelemetryData::default()
            };

            let output = analyzer.analyze(&telemetry, &session_info);

            // Property: For any telemetry with minimal throttle/brake, steering input,
            // and decreasing speed, the analyzer should detect understeer
            prop_assert!(!output.is_empty());
            let has_understeer = output.iter().any(|ann| matches!(
                ann,
                TelemetryAnnotation::MidCornerUndersteer { is_understeer: true, .. }
            ));
            prop_assert!(has_understeer);
        }
    }
}
