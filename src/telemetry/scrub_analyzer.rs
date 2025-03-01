use std::collections::HashMap;

use simple_moving_average::{SumTreeSMA, SMA};

use super::{TelemetryAnalyzer, TelemetryAnnotation};

/// only look for scrubbing or collect data points when steering is > than this %
const MIN_STEERING_PCT_MEASURE: f32 = 0.1;
const MIN_BRAKE_PCT_MEASURE: f32 = 0.4;
const MAX_THROTTLE_PCT_MEASURE: f32 = 0.4;

pub(crate) const SCRUBBING_YAW_RATE_ANNOTATION: &str = "scrubbing_average_yaw_change";
pub(crate) const YAW_RATE_CHANGE_ANNOTATION: &str = "yaw_change";
pub(crate) const SCRUBBING_ANNOTATION: &str = "is_scrubbing";

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
    ) -> std::collections::HashMap<String, super::TelemetryAnnotation> {
        let mut output = HashMap::new();
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
                    output.insert(
                        SCRUBBING_YAW_RATE_ANNOTATION.to_string(),
                        TelemetryAnnotation::Float(avg_steering_to_yaw_change),
                    );
                    output.insert(
                        YAW_RATE_CHANGE_ANNOTATION.to_string(),
                        TelemetryAnnotation::Float(yaw_rate_change),
                    );
                    output.insert(
                        SCRUBBING_ANNOTATION.to_string(),
                        TelemetryAnnotation::Bool(true),
                    );
                }
            }
        }

        output
    }
}
