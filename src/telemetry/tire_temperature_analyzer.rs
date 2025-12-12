use std::collections::VecDeque;

use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

/// Optimal tire temperature range (in Celsius)
/// Based on typical GT3 tire operating temperatures
const OPTIMAL_TEMP_MIN: f32 = 80.0;
const OPTIMAL_TEMP_MAX: f32 = 95.0;

/// Duration to track temperature history (in seconds)
const HISTORY_DURATION_S: usize = 60;

/// Minimum number of samples before detection
const MIN_SAMPLES: usize = 10;

/// Telemetry sample rate assumption (Hz)
const SAMPLE_RATE_HZ: f32 = 60.0;

#[derive(Clone, Debug)]
struct TireTemperatureSnapshot {
    timestamp_ms: u128,
    avg_temp: f32,
}

pub(crate) struct TireTemperatureAnalyzer {
    temp_history: VecDeque<TireTemperatureSnapshot>,
    history_duration_s: usize,
    optimal_temp_range: (f32, f32),
    sample_counter: usize,
    sample_interval: usize,
}

impl TireTemperatureAnalyzer {
    pub(crate) fn new() -> Self {
        Self::with_config(HISTORY_DURATION_S, (OPTIMAL_TEMP_MIN, OPTIMAL_TEMP_MAX))
    }

    pub(crate) fn with_config(history_duration_s: usize, optimal_temp_range: (f32, f32)) -> Self {
        // Sample every N telemetry points to avoid excessive memory usage
        // At 60Hz, sampling every 60 points = 1 sample per second
        let sample_interval = SAMPLE_RATE_HZ as usize;

        Self {
            temp_history: VecDeque::new(),
            history_duration_s,
            optimal_temp_range,
            sample_counter: 0,
            sample_interval,
        }
    }

    /// Calculate average temperature across all four tires
    fn calculate_avg_tire_temp(&self, telemetry: &TelemetryData) -> Option<f32> {
        let lf = telemetry.lf_tire_info.as_ref()?;
        let rf = telemetry.rf_tire_info.as_ref()?;
        let lr = telemetry.lr_tire_info.as_ref()?;
        let rr = telemetry.rr_tire_info.as_ref()?;

        // Average surface temperatures across all tires
        // Using surface temps as they're more representative of grip levels
        let temps = [
            lf.left_surface_temp,
            lf.middle_surface_temp,
            lf.right_surface_temp,
            rf.left_surface_temp,
            rf.middle_surface_temp,
            rf.right_surface_temp,
            lr.left_surface_temp,
            lr.middle_surface_temp,
            lr.right_surface_temp,
            rr.left_surface_temp,
            rr.middle_surface_temp,
            rr.right_surface_temp,
        ];

        let sum: f32 = temps.iter().sum();
        Some(sum / temps.len() as f32)
    }

    /// Check if sustained overheating is occurring
    fn check_overheating(&self) -> Option<TelemetryAnnotation> {
        if self.temp_history.len() < MIN_SAMPLES {
            return None;
        }

        // Calculate average temperature over the history window
        let sum: f32 = self.temp_history.iter().map(|s| s.avg_temp).sum();
        let avg_temp = sum / self.temp_history.len() as f32;

        // Check if sustained above optimal range
        if avg_temp > self.optimal_temp_range.1 {
            Some(TelemetryAnnotation::TireOverheating {
                avg_temp,
                optimal_max: self.optimal_temp_range.1,
                is_overheating: true,
            })
        } else {
            None
        }
    }

    /// Check if tires are too cold
    fn check_cold_tires(&self) -> Option<TelemetryAnnotation> {
        if self.temp_history.len() < MIN_SAMPLES {
            return None;
        }

        // Calculate average temperature over the history window
        let sum: f32 = self.temp_history.iter().map(|s| s.avg_temp).sum();
        let avg_temp = sum / self.temp_history.len() as f32;

        // Check if sustained below optimal range
        if avg_temp < self.optimal_temp_range.0 {
            Some(TelemetryAnnotation::TireCold {
                avg_temp,
                optimal_min: self.optimal_temp_range.0,
                is_cold: true,
            })
        } else {
            None
        }
    }
}

impl TelemetryAnalyzer for TireTemperatureAnalyzer {
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

        // Increment sample counter
        self.sample_counter += 1;

        // Only sample at the specified interval
        if !self.sample_counter.is_multiple_of(self.sample_interval) {
            return output;
        }

        // Calculate average tire temperature
        let avg_temp = match self.calculate_avg_tire_temp(telemetry) {
            Some(temp) => temp,
            None => return output, // No tire data available
        };

        // Add snapshot to history
        let snapshot = TireTemperatureSnapshot {
            timestamp_ms: telemetry.timestamp_ms,
            avg_temp,
        };
        self.temp_history.push_back(snapshot);

        // Remove old samples outside the history window
        let cutoff_time = telemetry
            .timestamp_ms
            .saturating_sub((self.history_duration_s as u128) * 1000);
        while let Some(oldest) = self.temp_history.front() {
            if oldest.timestamp_ms < cutoff_time {
                self.temp_history.pop_front();
            } else {
                break;
            }
        }

        // Check for overheating
        if let Some(annotation) = self.check_overheating() {
            output.push(annotation);
        }

        // Check for cold tires
        if let Some(annotation) = self.check_cold_tires() {
            output.push(annotation);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryData, TireInfo};
    use proptest::prelude::*;

    fn create_tire_info(temp: f32) -> TireInfo {
        TireInfo {
            left_carcass_temp: temp,
            middle_carcass_temp: temp,
            right_carcass_temp: temp,
            left_surface_temp: temp,
            middle_surface_temp: temp,
            right_surface_temp: temp,
        }
    }

    fn create_telemetry_with_tire_temp(temp: f32, timestamp_ms: u128) -> TelemetryData {
        let tire_info = create_tire_info(temp);
        TelemetryData {
            timestamp_ms,
            lf_tire_info: Some(tire_info.clone()),
            rf_tire_info: Some(tire_info.clone()),
            lr_tire_info: Some(tire_info.clone()),
            rr_tire_info: Some(tire_info),
            speed_mps: Some(10.),
            ..TelemetryData::default()
        }
    }

    #[test]
    fn test_overheating_detection_with_sustained_high_temps() {
        // Use 15 second history to ensure we can accumulate MIN_SAMPLES (10) samples
        let mut analyzer = TireTemperatureAnalyzer::with_config(15, (80.0, 95.0));
        let session_info = SessionInfo::default();

        // Send sustained high temperature samples
        // Need to send enough samples to fill the history window
        // At sample_interval = 60, we need 60 * MIN_SAMPLES telemetry points
        let high_temp = 100.0; // Above optimal max of 95.0
        let mut timestamp_ms = 0u128;

        for i in 0..1000 {
            let telemetry = create_telemetry_with_tire_temp(high_temp, timestamp_ms);
            let output = analyzer.analyze(&telemetry, &session_info);

            // After sufficient samples, should detect overheating
            if i >= 60 * MIN_SAMPLES && !output.is_empty() {
                assert_eq!(output.len(), 1);
                match &output[0] {
                    TelemetryAnnotation::TireOverheating {
                        avg_temp,
                        optimal_max,
                        is_overheating,
                    } => {
                        assert!(*is_overheating);
                        assert!(*avg_temp > *optimal_max);
                        assert_eq!(*optimal_max, 95.0);
                    }
                    _ => panic!("Expected TireOverheating annotation"),
                }
                return; // Test passed
            }

            timestamp_ms += 16; // ~60Hz
        }
        panic!("Failed to detect overheating");
    }

    #[test]
    fn test_cold_tire_detection_with_sustained_low_temps() {
        // Use 15 second history to ensure we can accumulate MIN_SAMPLES (10) samples
        let mut analyzer = TireTemperatureAnalyzer::with_config(15, (80.0, 95.0));
        let session_info = SessionInfo::default();

        // Send sustained low temperature samples
        let low_temp = 70.0; // Below optimal min of 80.0
        let mut timestamp_ms = 0u128;

        for i in 0..1000 {
            let telemetry = create_telemetry_with_tire_temp(low_temp, timestamp_ms);
            let output = analyzer.analyze(&telemetry, &session_info);

            // After sufficient samples, should detect cold tires
            if i >= 60 * MIN_SAMPLES && !output.is_empty() {
                assert_eq!(output.len(), 1);
                match &output[0] {
                    TelemetryAnnotation::TireCold {
                        avg_temp,
                        optimal_min,
                        is_cold,
                    } => {
                        assert!(*is_cold);
                        assert!(*avg_temp < *optimal_min);
                        assert_eq!(*optimal_min, 80.0);
                    }
                    _ => panic!("Expected TireCold annotation"),
                }
                return; // Test passed
            }

            timestamp_ms += 16;
        }
        panic!("Failed to detect cold tires");
    }

    #[test]
    fn test_no_detection_with_optimal_temps() {
        // Use 15 second history to ensure we can accumulate MIN_SAMPLES (10) samples
        let mut analyzer = TireTemperatureAnalyzer::with_config(15, (80.0, 95.0));
        let session_info = SessionInfo::default();

        // Send optimal temperature samples
        let optimal_temp = 87.5; // Within range
        let mut timestamp_ms = 0u128;

        for _ in 0..1000 {
            let telemetry = create_telemetry_with_tire_temp(optimal_temp, timestamp_ms);
            let output = analyzer.analyze(&telemetry, &session_info);

            // Should never detect issues with optimal temps
            assert!(output.is_empty());

            timestamp_ms += 16;
        }
    }

    #[test]
    fn test_with_missing_tire_data() {
        let mut analyzer = TireTemperatureAnalyzer::new();
        let session_info = SessionInfo::default();

        // Telemetry without tire data
        let telemetry = TelemetryData {
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            ..TelemetryData::default()
        };

        for _ in 0..1000 {
            let output = analyzer.analyze(&telemetry, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_insufficient_samples() {
        // Use 15 second history
        let mut analyzer = TireTemperatureAnalyzer::with_config(15, (80.0, 95.0));
        let session_info = SessionInfo::default();

        // Send high temp but not enough samples
        let high_temp = 100.0;
        let mut timestamp_ms = 0u128;

        // Send fewer samples than MIN_SAMPLES * sample_interval
        for _ in 0..(MIN_SAMPLES * 60 - 1) {
            let telemetry = create_telemetry_with_tire_temp(high_temp, timestamp_ms);
            let output = analyzer.analyze(&telemetry, &session_info);
            // Should not detect with insufficient samples
            assert!(output.is_empty());
            timestamp_ms += 16;
        }
    }

    #[test]
    fn test_history_window_cleanup() {
        // Use 5 second history for this test to verify cleanup works
        let mut analyzer = TireTemperatureAnalyzer::with_config(5, (80.0, 95.0));
        let session_info = SessionInfo::default();

        // Fill history with old samples
        let mut timestamp_ms = 0u128;
        for _ in 0..1000 {
            let telemetry = create_telemetry_with_tire_temp(100.0, timestamp_ms);
            analyzer.analyze(&telemetry, &session_info);
            timestamp_ms += 16;
        }

        // Verify history is bounded
        // At 60Hz sample rate with 1 sample per second, 5 second window = 5 samples max
        assert!(analyzer.temp_history.len() <= 6); // Allow small buffer
    }

    #[test]
    fn test_partial_tire_data() {
        let mut analyzer = TireTemperatureAnalyzer::new();
        let session_info = SessionInfo::default();

        // Telemetry with only some tire data
        let tire_info = create_tire_info(100.0);
        let telemetry = TelemetryData {
            lf_tire_info: Some(tire_info.clone()),
            rf_tire_info: Some(tire_info),
            lr_tire_info: None, // Missing rear tires
            rr_tire_info: None,
            ..TelemetryData::default()
        };

        for _ in 0..1000 {
            let output = analyzer.analyze(&telemetry, &session_info);
            // Should not analyze with incomplete tire data
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_no_detection_with_pit_limiter_engaged() {
        // Use 15 second history
        let mut analyzer = TireTemperatureAnalyzer::with_config(15, (80.0, 95.0));
        let session_info = SessionInfo::default();

        // Send sustained high temperature samples with pit limiter engaged
        let high_temp = 100.0; // Above optimal max
        let mut timestamp_ms = 0u128;

        for _ in 0..1000 {
            let mut telemetry = create_telemetry_with_tire_temp(high_temp, timestamp_ms);
            telemetry.is_pit_limiter_engaged = Some(true); // Pit limiter engaged

            let output = analyzer.analyze(&telemetry, &session_info);
            // Should not detect anything when pit limiter is engaged
            assert!(output.is_empty());

            timestamp_ms += 16;
        }
    }

    // **Feature: setup-assistant, Property 18: Tire overheating detection**
    // **Validates: Requirements 14.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_tire_overheating_detection(
            overheating_temp in OPTIMAL_TEMP_MAX + 5.0f32..150.0f32,
        ) {
            // Use 15 second history to ensure we can accumulate MIN_SAMPLES (10) samples
            // At 1 sample per second, 15 seconds = 15 samples max
            let mut analyzer = TireTemperatureAnalyzer::with_config(15, (OPTIMAL_TEMP_MIN, OPTIMAL_TEMP_MAX));
            let session_info = SessionInfo::default();

            // Send sustained overheating temperature samples
            // We need to send enough samples to:
            // 1. Fill the history window (MIN_SAMPLES samples at sample_interval spacing)
            // 2. Ensure the average stays above optimal_max
            let mut timestamp_ms = 0u128;
            let mut detected = false;

            // Need at least MIN_SAMPLES * sample_interval iterations to get MIN_SAMPLES in history
            // Detection only happens on sampling iterations (every 60th)
            // So we need to run past the minimum and check on sampling iterations
            for i in 0..2000 {
                let telemetry = create_telemetry_with_tire_temp(overheating_temp, timestamp_ms);
                let output = analyzer.analyze(&telemetry, &session_info);

                // Check for detection on any iteration after we've accumulated enough samples
                if !output.is_empty() {
                    // Should only detect after sufficient samples
                    // Note: iteration 599 is the 600th iteration (0-indexed)
                    prop_assert!(i >= 60 * MIN_SAMPLES - 1,
                        "Detected too early at iteration {}", i);

                    prop_assert_eq!(output.len(), 1);
                    match &output[0] {
                        TelemetryAnnotation::TireOverheating {
                            avg_temp,
                            optimal_max,
                            is_overheating,
                        } => {
                            prop_assert!(*is_overheating);
                            prop_assert!(*avg_temp > *optimal_max);
                            prop_assert_eq!(*optimal_max, OPTIMAL_TEMP_MAX);
                            detected = true;
                        }
                        _ => prop_assert!(false, "Expected TireOverheating annotation"),
                    }
                    break;
                }

                timestamp_ms += 16;
            }

            // Property: For any sequence of telemetry with tire temperatures
            // consistently above optimal range, the analyzer should detect overheating
            prop_assert!(detected, "Failed to detect overheating with temp {}", overheating_temp);
        }
    }
}
