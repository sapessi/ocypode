use simple_moving_average::{SMA, SumTreeSMA};

use crate::telemetry::is_telemetry_point_analyzable;

use super::{TelemetryAnalyzer, TelemetryAnnotation, TelemetryData};

/// only look for scrubbing or collect data points when steering is > than this %
const MIN_STEERING_PCT_MEASURE: f32 = 0.1;
const MIN_BRAKE_PCT_MEASURE: f32 = 0.4;
const MAX_THROTTLE_PCT_MEASURE: f32 = 0.4;

pub(crate) struct ScrubAnalyzer<const WINDOW_SIZE: usize> {
    steering_to_yaw_average: SumTreeSMA<f32, f32, WINDOW_SIZE>,
    min_points: usize,
}

impl<const WINDOW_SIZE: usize> ScrubAnalyzer<WINDOW_SIZE> {
    pub(crate) fn new(min_points: usize) -> Self {
        Self {
            steering_to_yaw_average: SumTreeSMA::new(),
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
        let mut output = Vec::new();

        // Skip analysis if doesn't meet requirements
        if !is_telemetry_point_analyzable(telemetry) {
            return output;
        }

        // Extract data from TelemetryData
        let brake = telemetry.brake.unwrap_or(0.0);
        let throttle = telemetry.throttle.unwrap_or(0.0);
        let steering_pct = telemetry.steering_pct.unwrap_or(0.0);

        // Access yaw_rate_rps from TelemetryData, handle None gracefully
        let yaw_rate = match telemetry.yaw_rate_rps {
            Some(rate) => rate,
            None => {
                // If yaw rate is not available, we cannot perform scrub analysis
                // Return empty output
                return output;
            }
        };

        let yaw_rate_change = steering_pct.abs() - yaw_rate.abs();
        if steering_pct > MIN_STEERING_PCT_MEASURE
            && (brake >= MIN_BRAKE_PCT_MEASURE || throttle <= MAX_THROTTLE_PCT_MEASURE)
        {
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
        }

        output
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
    fn test_no_scrub_annotation_when_yaw_rate_is_none() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = TelemetryData {
            brake: Some(0.5),
            throttle: Some(0.3),
            steering_pct: Some(0.2),
            yaw_rate_rps: None, // yaw_rate is not available
            ..TelemetryData::default()
        };
        let session_info = SessionInfo::default();

        // Even with sufficient samples, should return empty if yaw_rate is None
        for _ in 0..10 {
            let output = analyzer.analyze(&telemetry_data, &session_info);
            assert!(output.is_empty());
        }
    }
}
