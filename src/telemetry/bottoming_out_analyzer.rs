use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

/// Minimum pitch change (in radians) to consider for bottoming out detection
const MIN_PITCH_CHANGE_RAD: f32 = 0.05;
/// Maximum steering percentage to filter for straights or over bumps
const MAX_STEERING_PCT: f32 = 0.2;
/// Minimum speed loss (in m/s) to consider for bottoming out
const MIN_SPEED_LOSS_MPS: f32 = 0.5;

pub(crate) struct BottomingOutAnalyzer {
    prev_pitch: Option<f32>,
    prev_speed: Option<f32>,
}

impl BottomingOutAnalyzer {
    pub(crate) fn new() -> Self {
        Self {
            prev_pitch: None,
            prev_speed: None,
        }
    }
}

impl TelemetryAnalyzer for BottomingOutAnalyzer {
    fn analyze(
        &mut self,
        telemetry: &TelemetryData,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Skip analysis if pit limiter is engaged (not at racing speed)
        if telemetry.is_pit_limiter_engaged.unwrap_or(false) {
            return output;
        }

        // Extract required fields from telemetry
        let pitch = match telemetry.pitch_rad {
            Some(p) => p,
            None => {
                // If pitch is not available, we cannot perform bottoming out analysis
                return output;
            }
        };

        let speed = match telemetry.speed_mps {
            Some(s) => s,
            None => {
                // If speed is not available, we cannot perform bottoming out analysis
                return output;
            }
        };

        let steering_pct = telemetry.steering_pct.unwrap_or(0.0);

        // Only analyze when steering is minimal (straights or over bumps)
        if steering_pct.abs() <= MAX_STEERING_PCT {
            // Check if we have previous state to compare
            if let (Some(prev_pitch), Some(prev_speed)) = (self.prev_pitch, self.prev_speed) {
                // Calculate pitch change (compression is negative pitch change)
                let pitch_change = (pitch - prev_pitch).abs();

                // Calculate speed loss
                let speed_loss = prev_speed - speed;

                // Detect bottoming out: sudden pitch change with speed loss
                if pitch_change > MIN_PITCH_CHANGE_RAD && speed_loss > MIN_SPEED_LOSS_MPS {
                    output.push(TelemetryAnnotation::BottomingOut {
                        pitch_change,
                        speed_loss,
                        is_bottoming: true,
                    });
                }
            }
        }

        // Update previous state
        self.prev_pitch = Some(pitch);
        self.prev_speed = Some(speed);

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryData};
    use proptest::prelude::*;

    #[test]
    fn test_bottoming_out_detected() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        // First telemetry point to establish baseline
        let telemetry1 = TelemetryData {
            pitch_rad: Some(0.0),
            speed_mps: Some(50.0),
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry1, &session_info);

        // Second telemetry point with sudden pitch change and speed loss
        let telemetry2 = TelemetryData {
            pitch_rad: Some(0.1),  // Significant pitch change
            speed_mps: Some(48.0), // Speed loss of 2.0 m/s
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry2, &session_info);
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::BottomingOut {
                pitch_change,
                speed_loss,
                is_bottoming,
            } => {
                assert!(*is_bottoming);
                assert!(*pitch_change > MIN_PITCH_CHANGE_RAD);
                assert!(*speed_loss > MIN_SPEED_LOSS_MPS);
            }
            _ => panic!("Expected BottomingOut annotation"),
        }
    }

    #[test]
    fn test_no_bottoming_with_high_steering() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        // First telemetry point
        let telemetry1 = TelemetryData {
            pitch_rad: Some(0.0),
            speed_mps: Some(50.0),
            steering_pct: Some(0.5), // High steering
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry1, &session_info);

        // Second telemetry point with pitch change and speed loss but high steering
        let telemetry2 = TelemetryData {
            pitch_rad: Some(0.1),
            speed_mps: Some(48.0),
            steering_pct: Some(0.5), // High steering - should filter out
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry2, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_bottoming_without_speed_loss() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        // First telemetry point
        let telemetry1 = TelemetryData {
            pitch_rad: Some(0.0),
            speed_mps: Some(50.0),
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry1, &session_info);

        // Second telemetry point with pitch change but no speed loss
        let telemetry2 = TelemetryData {
            pitch_rad: Some(0.1),
            speed_mps: Some(51.0), // Speed increased
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry2, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_bottoming_without_pitch_change() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        // First telemetry point
        let telemetry1 = TelemetryData {
            pitch_rad: Some(0.0),
            speed_mps: Some(50.0),
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry1, &session_info);

        // Second telemetry point with speed loss but minimal pitch change
        let telemetry2 = TelemetryData {
            pitch_rad: Some(0.01), // Small pitch change
            speed_mps: Some(48.0),
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry2, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_missing_pitch() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            pitch_rad: None, // Missing pitch
            speed_mps: Some(50.0),
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_missing_speed() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            pitch_rad: Some(0.0),
            speed_mps: None, // Missing speed
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_first_telemetry_point() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        // First telemetry point should not detect anything (no previous state)
        let telemetry = TelemetryData {
            pitch_rad: Some(0.1),
            speed_mps: Some(48.0),
            steering_pct: Some(0.0),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_pit_limiter_engaged() {
        let mut analyzer = BottomingOutAnalyzer::new();
        let session_info = SessionInfo::default();

        // First telemetry point to establish baseline
        let telemetry1 = TelemetryData {
            pitch_rad: Some(0.0),
            speed_mps: Some(50.0),
            steering_pct: Some(0.0),
            is_pit_limiter_engaged: Some(false),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry1, &session_info);

        // Second telemetry point with conditions that would trigger bottoming, but pit limiter engaged
        let telemetry2 = TelemetryData {
            pitch_rad: Some(0.1),  // Significant pitch change
            speed_mps: Some(48.0), // Speed loss
            steering_pct: Some(0.0),
            is_pit_limiter_engaged: Some(true), // Pit limiter engaged
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry2, &session_info);
        // Should not detect anything when pit limiter is engaged
        assert!(output.is_empty());
    }

    // **Feature: setup-assistant, Property 20: Bottoming out detection**
    // **Validates: Requirements 15.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_bottoming_out_detection(
            initial_pitch in -0.5f32..0.5f32,
            initial_speed in 20.0f32..100.0f32,
            pitch_change in MIN_PITCH_CHANGE_RAD + 0.01f32..0.3f32,
            speed_loss in MIN_SPEED_LOSS_MPS + 0.1f32..10.0f32,
            steering in 0.0f32..MAX_STEERING_PCT,
        ) {
            let mut analyzer = BottomingOutAnalyzer::new();
            let session_info = SessionInfo::default();

            // First telemetry point to establish baseline
            let telemetry1 = TelemetryData {
                pitch_rad: Some(initial_pitch),
                speed_mps: Some(initial_speed),
                steering_pct: Some(steering),
                ..TelemetryData::default()
            };
            analyzer.analyze(&telemetry1, &session_info);

            // Second telemetry point with sudden pitch change and speed loss
            let final_pitch = initial_pitch + pitch_change;
            let final_speed = initial_speed - speed_loss;
            let telemetry2 = TelemetryData {
                pitch_rad: Some(final_pitch),
                speed_mps: Some(final_speed),
                steering_pct: Some(steering),
                ..TelemetryData::default()
            };

            let output = analyzer.analyze(&telemetry2, &session_info);

            // Property: For any telemetry with sudden pitch change and speed loss
            // while steering is minimal, the analyzer should detect bottoming
            prop_assert_eq!(output.len(), 1);
            match &output[0] {
                TelemetryAnnotation::BottomingOut {
                    pitch_change: detected_pitch_change,
                    speed_loss: detected_speed_loss,
                    is_bottoming,
                } => {
                    prop_assert!(*is_bottoming);
                    prop_assert!(*detected_pitch_change >= MIN_PITCH_CHANGE_RAD);
                    prop_assert!(*detected_speed_loss >= MIN_SPEED_LOSS_MPS);
                }
                _ => prop_assert!(false, "Expected BottomingOut annotation"),
            }
        }
    }
}
