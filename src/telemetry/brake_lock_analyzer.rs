use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

/// Minimum brake percentage to consider for brake lock detection
const MIN_BRAKE_PCT: f32 = 0.3;

pub(crate) struct BrakeLockAnalyzer {
    abs_activation_count: usize,
    in_braking_zone: bool,
    prev_brake: f32,
}

impl BrakeLockAnalyzer {
    pub(crate) fn new() -> Self {
        Self {
            abs_activation_count: 0,
            in_braking_zone: false,
            prev_brake: 0.0,
        }
    }
}

impl TelemetryAnalyzer for BrakeLockAnalyzer {
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
        let is_abs_active = telemetry.is_abs_active.unwrap_or(false);

        // Detect braking zone entry and exit
        if brake > MIN_BRAKE_PCT && self.prev_brake <= MIN_BRAKE_PCT {
            // Entering braking zone
            self.in_braking_zone = true;
            self.abs_activation_count = 0;
        } else if brake <= MIN_BRAKE_PCT && self.prev_brake > MIN_BRAKE_PCT {
            // Exiting braking zone
            self.in_braking_zone = false;
            self.abs_activation_count = 0;
        }

        // Track ABS activations during braking
        if self.in_braking_zone && is_abs_active {
            self.abs_activation_count += 1;

            // Detect brake locking when ABS is active
            // Try to classify as front or rear lock based on tire slip data if available
            // Note: Currently tire slip data is not available in TelemetryData
            // So we'll create a general brake lock annotation

            // Check if we have tire info that could indicate slip
            // (In the future, if tire slip data becomes available, we can use it here)
            let has_tire_data = telemetry.lf_tire_info.is_some()
                && telemetry.rf_tire_info.is_some()
                && telemetry.lr_tire_info.is_some()
                && telemetry.rr_tire_info.is_some();

            if has_tire_data {
                // For now, we don't have actual tire slip data in TireInfo
                // TireInfo only contains temperature data
                // So we'll create a general annotation without front/rear classification
                // When tire slip data becomes available, we can enhance this logic

                // Placeholder for future tire slip-based classification
                // For now, we'll just detect that brake lock occurred
                output.push(TelemetryAnnotation::FrontBrakeLock {
                    abs_activation_count: self.abs_activation_count,
                    is_front_lock: false, // Cannot determine without slip data
                });
            } else {
                // No tire data available, create general brake lock annotation
                // We'll use FrontBrakeLock with is_front_lock = false to indicate
                // we detected brake lock but cannot classify it
                output.push(TelemetryAnnotation::FrontBrakeLock {
                    abs_activation_count: self.abs_activation_count,
                    is_front_lock: false,
                });
            }
        }

        self.prev_brake = brake;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryData};
    use proptest::prelude::*;

    #[test]
    fn test_abs_activation_detected() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        // Enter braking zone
        let telemetry = TelemetryData {
            brake: Some(0.5),
            is_abs_active: Some(false),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry, &session_info);

        // ABS activates
        let telemetry = TelemetryData {
            brake: Some(0.8),
            is_abs_active: Some(true),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::FrontBrakeLock {
                abs_activation_count,
                is_front_lock: _,
            } => {
                assert_eq!(*abs_activation_count, 1);
            }
            _ => panic!("Expected FrontBrakeLock annotation"),
        }
    }

    #[test]
    fn test_no_detection_without_braking() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        // ABS active but no braking
        let telemetry = TelemetryData {
            brake: Some(0.1), // Below threshold
            is_abs_active: Some(true),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_without_abs() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        // Braking but no ABS
        let telemetry = TelemetryData {
            brake: Some(0.8),
            is_abs_active: Some(false),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_abs_activation_count_increments() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        // Enter braking zone
        let telemetry = TelemetryData {
            brake: Some(0.5),
            is_abs_active: Some(false),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry, &session_info);

        // Multiple ABS activations
        for i in 1..=5 {
            let telemetry = TelemetryData {
                brake: Some(0.8),
                is_abs_active: Some(true),
                speed_mps: Some(10.),
                ..TelemetryData::default()
            };

            let output = analyzer.analyze(&telemetry, &session_info);
            assert_eq!(output.len(), 1);
            match &output[0] {
                TelemetryAnnotation::FrontBrakeLock {
                    abs_activation_count,
                    is_front_lock: _,
                } => {
                    assert_eq!(*abs_activation_count, i);
                }
                _ => panic!("Expected FrontBrakeLock annotation"),
            }
        }
    }

    #[test]
    fn test_braking_zone_reset() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        // First braking zone
        let telemetry = TelemetryData {
            brake: Some(0.5),
            is_abs_active: Some(false),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry, &session_info);

        // ABS activates
        let telemetry = TelemetryData {
            brake: Some(0.8),
            is_abs_active: Some(true),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        let output = analyzer.analyze(&telemetry, &session_info);
        assert_eq!(output.len(), 1);

        // Exit braking zone
        let telemetry = TelemetryData {
            brake: Some(0.1),
            is_abs_active: Some(false),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry, &session_info);

        // Enter new braking zone
        let telemetry = TelemetryData {
            brake: Some(0.5),
            is_abs_active: Some(false),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry, &session_info);

        // ABS activates in new zone - count should reset
        let telemetry = TelemetryData {
            brake: Some(0.8),
            is_abs_active: Some(true),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        };
        let output = analyzer.analyze(&telemetry, &session_info);
        assert_eq!(output.len(), 1);
        match &output[0] {
            TelemetryAnnotation::FrontBrakeLock {
                abs_activation_count,
                is_front_lock: _,
            } => {
                assert_eq!(*abs_activation_count, 1); // Reset to 1
            }
            _ => panic!("Expected FrontBrakeLock annotation"),
        }
    }

    #[test]
    fn test_with_missing_brake_data() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            brake: None,
            is_abs_active: Some(true),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_with_missing_abs_data() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        let telemetry = TelemetryData {
            brake: Some(0.8),
            is_abs_active: None,
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_detection_with_pit_limiter_engaged() {
        let mut analyzer = BrakeLockAnalyzer::new();
        let session_info = SessionInfo::default();

        // Enter braking zone
        let telemetry = TelemetryData {
            brake: Some(0.5),
            is_abs_active: Some(false),
            is_pit_limiter_engaged: Some(false),
            ..TelemetryData::default()
        };
        analyzer.analyze(&telemetry, &session_info);

        // ABS activates but pit limiter is engaged
        let telemetry = TelemetryData {
            brake: Some(0.8),
            is_abs_active: Some(true),
            is_pit_limiter_engaged: Some(true), // Pit limiter engaged
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&telemetry, &session_info);
        // Should not detect anything when pit limiter is engaged
        assert!(output.is_empty());
    }

    // **Feature: setup-assistant, Property 16: Brake lock detection**
    // **Validates: Requirements 13.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_brake_lock_detection(
            brake_level in MIN_BRAKE_PCT + 0.01f32..1.0f32,
        ) {
            let mut analyzer = BrakeLockAnalyzer::new();
            let session_info = SessionInfo::default();

            // Enter braking zone
            let telemetry = TelemetryData {
                brake: Some(brake_level),
                is_abs_active: Some(false),
                speed_mps: Some(10.),
                ..TelemetryData::default()
            };
            analyzer.analyze(&telemetry, &session_info);

            // ABS activates during braking
            let telemetry = TelemetryData {
                brake: Some(brake_level),
                is_abs_active: Some(true),
                speed_mps: Some(10.),
                ..TelemetryData::default()
            };

            let output = analyzer.analyze(&telemetry, &session_info);

            // Property: For any telemetry with ABS active and brake application above threshold,
            // the analyzer should detect brake locking
            prop_assert_eq!(output.len(), 1);
            match &output[0] {
                TelemetryAnnotation::FrontBrakeLock {
                    abs_activation_count,
                    is_front_lock: _,
                } => {
                    prop_assert!(*abs_activation_count >= 1);
                }
                _ => prop_assert!(false, "Expected FrontBrakeLock annotation"),
            }
        }
    }
}
