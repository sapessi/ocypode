use simple_moving_average::{SMA, SumTreeSMA};

use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData, TireInfo};

/// only look for scrubbing or collect data points when steering is > than this %
const MIN_STEERING_PCT_MEASURE: f32 = 0.1;
const MIN_BRAKE_PCT_MEASURE: f32 = 0.4;
const MAX_THROTTLE_PCT_MEASURE: f32 = 0.4;
/// Minimum speed to consider for scrub analysis (m/s)
const MIN_SPEED_MPS: f32 = 5.0;
/// Temperature difference threshold indicating scrubbing (°C)
const SCRUB_TEMP_THRESHOLD: f32 = 5.0;

pub(crate) struct ScrubAnalyzer<const WINDOW_SIZE: usize> {
    // For yaw rate based analysis (when available)
    steering_to_yaw_average: SumTreeSMA<f32, f32, WINDOW_SIZE>,
    // For tire temperature based analysis (fallback for ACC)
    tire_temp_baseline: SumTreeSMA<f32, f32, WINDOW_SIZE>,
    min_points: usize,
}

impl<const WINDOW_SIZE: usize> ScrubAnalyzer<WINDOW_SIZE> {
    pub(crate) fn new(min_points: usize) -> Self {
        Self {
            steering_to_yaw_average: SumTreeSMA::new(),
            tire_temp_baseline: SumTreeSMA::new(),
            min_points,
        }
    }
}

impl<const WINDOW_SIZE: usize> TelemetryAnalyzer for ScrubAnalyzer<WINDOW_SIZE> {
    fn analyze(
        &mut self,
        telemetry: &TelemetryData,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let output = Vec::new();

        // Skip analysis if doesn't meet requirements
        if !is_telemetry_point_analyzable(telemetry) {
            return output;
        }

        // Extract common data from TelemetryData
        let brake = telemetry.brake.unwrap_or(0.0);
        let throttle = telemetry.throttle.unwrap_or(0.0);
        let steering_pct = telemetry.steering_pct.unwrap_or(0.0);
        let speed_mps = telemetry.speed_mps.unwrap_or(0.0);

        // Only analyze when conditions are right for scrub detection
        if steering_pct.abs() <= MIN_STEERING_PCT_MEASURE
            || speed_mps < MIN_SPEED_MPS
            || (brake < MIN_BRAKE_PCT_MEASURE && throttle > MAX_THROTTLE_PCT_MEASURE)
        {
            return output;
        }

        // Try yaw rate based analysis first (for iRacing)
        if let Some(yaw_rate) = telemetry.yaw_rate_rps {
            return self.analyze_with_yaw_rate(steering_pct, yaw_rate);
        }

        // Fallback to tire temperature based analysis (for ACC)
        self.analyze_with_tire_temperature(telemetry, steering_pct)
    }
}

impl<const WINDOW_SIZE: usize> ScrubAnalyzer<WINDOW_SIZE> {
    /// Analyze scrubbing using yaw rate data (original method for iRacing)
    fn analyze_with_yaw_rate(
        &mut self,
        steering_pct: f32,
        yaw_rate: f32,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        let yaw_rate_change = steering_pct.abs() - yaw_rate.abs();

        // calculate relationship between two points
        self.steering_to_yaw_average.add_sample(yaw_rate_change);

        // we are collected enough points, let's see if we are scrubbing
        if self.steering_to_yaw_average.get_num_samples() >= self.min_points {
            let avg_steering_to_yaw_change = self.steering_to_yaw_average.get_average();
            if yaw_rate_change > avg_steering_to_yaw_change {
                output.push(TelemetryAnnotation::Scrub {
                    avg_yaw_rate_change: avg_steering_to_yaw_change,
                    cur_yaw_rate_change: yaw_rate_change,
                    is_scrubbing: true,
                });
            }
        }

        output
    }

    /// Analyze scrubbing using tire temperature data (fallback for ACC)
    fn analyze_with_tire_temperature(
        &mut self,
        telemetry: &TelemetryData,
        _steering_pct: f32,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Calculate average tire temperature across all tires
        let avg_tire_temp = self.calculate_average_tire_temperature(telemetry);
        if avg_tire_temp <= 0.0 {
            return output; // No valid tire temperature data
        }

        // Build baseline of tire temperatures during cornering
        self.tire_temp_baseline.add_sample(avg_tire_temp);

        // Once we have enough samples, check for temperature spikes indicating scrubbing
        if self.tire_temp_baseline.get_num_samples() >= self.min_points {
            let baseline_temp = self.tire_temp_baseline.get_average();
            let temp_increase = avg_tire_temp - baseline_temp;

            // Detect scrubbing: significant temperature increase above baseline
            if temp_increase > SCRUB_TEMP_THRESHOLD {
                // Use temperature-based values for the annotation
                // Map temperature increase to a yaw rate change equivalent for consistency
                let simulated_yaw_change = temp_increase / 10.0; // Scale factor for display

                output.push(TelemetryAnnotation::Scrub {
                    avg_yaw_rate_change: baseline_temp / 10.0, // Baseline as reference
                    cur_yaw_rate_change: simulated_yaw_change,
                    is_scrubbing: true,
                });
            }
        }

        output
    }

    /// Calculate average tire temperature across all available tires
    fn calculate_average_tire_temperature(&self, telemetry: &TelemetryData) -> f32 {
        let mut total_temp = 0.0;
        let mut tire_count = 0;

        // Helper to add tire temperatures if available
        let add_tire_temp = |total: &mut f32, count: &mut i32, tire_info: &Option<TireInfo>| {
            if let Some(tire) = tire_info {
                // Use average of surface temperatures as they're more responsive to scrubbing
                let tire_avg =
                    (tire.left_surface_temp + tire.middle_surface_temp + tire.right_surface_temp)
                        / 3.0;
                if tire_avg > 0.0 {
                    // Only count valid temperatures
                    *total += tire_avg;
                    *count += 1;
                }
            }
        };

        add_tire_temp(&mut total_temp, &mut tire_count, &telemetry.lf_tire_info);
        add_tire_temp(&mut total_temp, &mut tire_count, &telemetry.rf_tire_info);
        add_tire_temp(&mut total_temp, &mut tire_count, &telemetry.lr_tire_info);
        add_tire_temp(&mut total_temp, &mut tire_count, &telemetry.rr_tire_info);

        if tire_count > 0 {
            total_temp / tire_count as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionInfo, TelemetryData};

    #[test]
    fn test_no_scrub_annotation_due_to_insufficient_points() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = TelemetryData {
            brake: Some(0.5),
            throttle: Some(0.3),
            steering_pct: Some(0.2),
            yaw_rate_rps: Some(0.1),
            speed_mps: Some(10.0), // Above MIN_SPEED_MPS
            is_pit_limiter_engaged: Some(false),
            ..TelemetryData::default()
        };
        let session_info = SessionInfo::default();

        // Add fewer samples than the minimum points threshold
        for _ in 0..4 {
            analyzer.analyze(&telemetry_data, &session_info);
        }

        let output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_scrub_annotation_due_to_high_throttle() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = TelemetryData {
            brake: Some(0.1),
            throttle: Some(0.5),
            steering_pct: Some(0.2),
            yaw_rate_rps: Some(0.1),
            speed_mps: Some(10.0), // Above MIN_SPEED_MPS
            is_pit_limiter_engaged: Some(false),
            ..TelemetryData::default()
        };
        let session_info = SessionInfo::default();

        // Add enough samples to reach the minimum points threshold
        for _ in 0..5 {
            analyzer.analyze(&telemetry_data, &session_info);
        }

        let output = analyzer.analyze(&telemetry_data, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_scrub_annotation_due_to_low_speed() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = TelemetryData {
            brake: Some(0.5),
            throttle: Some(0.3),
            steering_pct: Some(0.2),
            yaw_rate_rps: Some(0.1),
            speed_mps: Some(2.0), // Below MIN_SPEED_MPS (5.0)
            is_pit_limiter_engaged: Some(false),
            ..TelemetryData::default()
        };
        let session_info = SessionInfo::default();

        // Should return empty due to low speed
        for _ in 0..10 {
            let output = analyzer.analyze(&telemetry_data, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_no_scrub_annotation_when_no_tire_data_and_no_yaw_rate() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = TelemetryData {
            brake: Some(0.5),
            throttle: Some(0.3),
            steering_pct: Some(0.2),
            yaw_rate_rps: None,    // No yaw_rate
            speed_mps: Some(10.0), // Above MIN_SPEED_MPS
            is_pit_limiter_engaged: Some(false),
            // No tire info provided
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            ..TelemetryData::default()
        };
        let session_info = SessionInfo::default();

        // Should return empty when neither yaw rate nor tire data is available
        for _ in 0..10 {
            let output = analyzer.analyze(&telemetry_data, &session_info);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn test_scrub_annotation_produced_with_yaw_rate() {
        let mut analyzer = ScrubAnalyzer::<10>::new(3);
        let session_info = SessionInfo::default();

        // Build up the moving average with baseline data (lower yaw rate change)
        let baseline_telemetry = TelemetryData {
            brake: Some(0.5),                    // Above MIN_BRAKE_PCT_MEASURE (0.4)
            throttle: Some(0.2),                 // Below MAX_THROTTLE_PCT_MEASURE (0.4)
            steering_pct: Some(0.2),             // Above MIN_STEERING_PCT_MEASURE (0.1)
            yaw_rate_rps: Some(0.15),            // Yaw rate change = 0.2 - 0.15 = 0.05
            speed_mps: Some(20.0),               // Above MIN_SPEED_MPS (5.0)
            is_pit_limiter_engaged: Some(false), // Not in pit limiter
            ..TelemetryData::default()
        };

        // Build up the moving average with baseline data
        for _ in 0..3 {
            analyzer.analyze(&baseline_telemetry, &session_info);
        }

        // Create a data point with higher yaw rate change that should trigger scrub annotation
        let scrub_telemetry = TelemetryData {
            brake: Some(0.6),
            throttle: Some(0.1),
            steering_pct: Some(0.4),             // Higher steering
            yaw_rate_rps: Some(0.05),            // Lower yaw rate response
            speed_mps: Some(25.0),               // Above MIN_SPEED_MPS (5.0)
            is_pit_limiter_engaged: Some(false), // Not in pit limiter
            // Yaw rate change = 0.4 - 0.05 = 0.35 (much higher than baseline 0.05)
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&scrub_telemetry, &session_info);

        // Should produce a scrub annotation
        assert!(!output.is_empty());
        assert_eq!(output.len(), 1);

        match &output[0] {
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change,
                cur_yaw_rate_change,
                is_scrubbing,
            } => {
                assert!(*is_scrubbing);
                assert!(*cur_yaw_rate_change > *avg_yaw_rate_change);
                // Current yaw rate change should be steering_pct.abs() - yaw_rate_rps.abs()
                // = 0.4 - 0.05 = 0.35
                assert!((cur_yaw_rate_change - 0.35).abs() < 0.001);
                // Average should be reasonable (includes baseline 0.05 and current 0.35)
                assert!(*avg_yaw_rate_change > 0.0);
                assert!(*avg_yaw_rate_change < *cur_yaw_rate_change);
            }
            _ => panic!("Expected Scrub annotation, got {:?}", output[0]),
        }
    }

    #[test]
    fn test_scrub_annotation_produced_with_tire_temperature() {
        let mut analyzer = ScrubAnalyzer::<10>::new(3);
        let session_info = SessionInfo::default();

        // Create tire info with baseline temperatures
        let baseline_tire_info = TireInfo {
            left_carcass_temp: 80.0,
            middle_carcass_temp: 80.0,
            right_carcass_temp: 80.0,
            left_surface_temp: 85.0,
            middle_surface_temp: 85.0,
            right_surface_temp: 85.0,
        };

        // Build up baseline tire temperature data
        let baseline_telemetry = TelemetryData {
            brake: Some(0.5),                    // Above MIN_BRAKE_PCT_MEASURE (0.4)
            throttle: Some(0.2),                 // Below MAX_THROTTLE_PCT_MEASURE (0.4)
            steering_pct: Some(0.2),             // Above MIN_STEERING_PCT_MEASURE (0.1)
            yaw_rate_rps: None,                  // No yaw rate data (ACC scenario)
            speed_mps: Some(20.0),               // Above MIN_SPEED_MPS (5.0)
            is_pit_limiter_engaged: Some(false), // Not in pit limiter
            lf_tire_info: Some(baseline_tire_info.clone()),
            rf_tire_info: Some(baseline_tire_info.clone()),
            lr_tire_info: Some(baseline_tire_info.clone()),
            rr_tire_info: Some(baseline_tire_info.clone()),
            ..TelemetryData::default()
        };

        // Build up the baseline
        for _ in 0..3 {
            analyzer.analyze(&baseline_telemetry, &session_info);
        }

        // Create tire info with elevated temperatures (indicating scrubbing)
        let hot_tire_info = TireInfo {
            left_carcass_temp: 80.0,
            middle_carcass_temp: 80.0,
            right_carcass_temp: 80.0,
            left_surface_temp: 95.0,   // +10°C increase
            middle_surface_temp: 95.0, // +10°C increase
            right_surface_temp: 95.0,  // +10°C increase
        };

        // Create a data point with elevated tire temperatures
        let scrub_telemetry = TelemetryData {
            brake: Some(0.6),
            throttle: Some(0.1),
            steering_pct: Some(0.4),             // Higher steering
            yaw_rate_rps: None,                  // No yaw rate data (ACC scenario)
            speed_mps: Some(25.0),               // Above MIN_SPEED_MPS (5.0)
            is_pit_limiter_engaged: Some(false), // Not in pit limiter
            lf_tire_info: Some(hot_tire_info.clone()),
            rf_tire_info: Some(hot_tire_info.clone()),
            lr_tire_info: Some(hot_tire_info.clone()),
            rr_tire_info: Some(hot_tire_info.clone()),
            ..TelemetryData::default()
        };

        let output = analyzer.analyze(&scrub_telemetry, &session_info);

        // Should produce a scrub annotation
        assert!(!output.is_empty());
        assert_eq!(output.len(), 1);

        match &output[0] {
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: _,
                cur_yaw_rate_change: _,
                is_scrubbing,
            } => {
                assert!(*is_scrubbing);
                // For temperature-based detection, we don't validate the exact values
                // as they're simulated for consistency with the yaw rate approach
            }
            _ => panic!("Expected Scrub annotation, got {:?}", output[0]),
        }
    }
}
