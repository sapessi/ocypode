use simple_moving_average::{SumTreeSMA, SMA};

use super::{TelemetryAnalyzer, TelemetryAnnotation};

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
        telemetry_point: &super::TelemetryPoint,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();
        let yaw_rate_change = telemetry_point.steering_pct.abs() - telemetry_point.yaw_rate.abs();
        if telemetry_point.steering_pct > MIN_STEERING_PCT_MEASURE
            && (telemetry_point.brake >= MIN_BRAKE_PCT_MEASURE
                || telemetry_point.throttle <= MAX_THROTTLE_PCT_MEASURE)
        {
            // calcualte relationship between two points
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
    use crate::telemetry::{SessionInfo, TelemetryAnnotation, TelemetryPoint};

    #[test]
    fn test_scrub_annotation_inserted() {
        let mut analyzer = ScrubAnalyzer::<5>::new(5);
        let telemetry_point = TelemetryPoint {
            steering_pct: 0.2,
            brake: 0.5,
            throttle: 0.3,
            yaw_rate: 0.1,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Add enough samples to reach the minimum points threshold
        for _ in 0..5 {
            analyzer.analyze(&telemetry_point, &session_info);
        }

        let scrub_telemetry_point = TelemetryPoint {
            steering_pct: 0.5,
            brake: 0.5,
            throttle: 0.3,
            yaw_rate: 0.04,
            ..Default::default()
        };
        let output = analyzer.analyze(&scrub_telemetry_point, &session_info);
        assert_eq!(output.len(), 1);
        let scrub_annotation = match output.first().unwrap() {
            TelemetryAnnotation::Scrub {
                avg_yaw_rate_change: _,
                cur_yaw_rate_change: _,
                is_scrubbing,
            } => *is_scrubbing,
            _ => false,
        };
        assert!(scrub_annotation);
    }

    #[test]
    fn test_no_scrub_annotation_due_to_insufficient_points() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_point = TelemetryPoint {
            steering_pct: 0.2,
            brake: 0.5,
            throttle: 0.3,
            yaw_rate: 0.1,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Add fewer samples than the minimum points threshold
        for _ in 0..4 {
            analyzer.analyze(&telemetry_point, &session_info);
        }

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_scrub_annotation_due_to_low_steering() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_point = TelemetryPoint {
            steering_pct: 0.05,
            brake: 0.5,
            throttle: 0.3,
            yaw_rate: 0.1,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Add enough samples to reach the minimum points threshold
        for _ in 0..5 {
            analyzer.analyze(&telemetry_point, &session_info);
        }

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_scrub_annotation_due_to_high_throttle() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_point = TelemetryPoint {
            steering_pct: 0.2,
            brake: 0.1,
            throttle: 0.5,
            yaw_rate: 0.1,
            ..Default::default()
        };
        let session_info = SessionInfo::default();

        // Add enough samples to reach the minimum points threshold
        for _ in 0..5 {
            analyzer.analyze(&telemetry_point, &session_info);
        }

        let output = analyzer.analyze(&telemetry_point, &session_info);
        assert!(output.is_empty());
    }
}
