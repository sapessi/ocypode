use simple_moving_average::{SMA, SumTreeSMA};
use simetry::Moment;

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
        telemetry: &dyn Moment,
        _session_info: &super::SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();
        
        // Extract data from Moment trait
        let pedals = telemetry.pedals();
        let brake = pedals.as_ref().map(|p| p.brake as f32).unwrap_or(0.0);
        let throttle = pedals.as_ref().map(|p| p.throttle as f32).unwrap_or(0.0);
        
        // Note: steering_pct and yaw_rate are not available in the base Moment trait
        // For now, we'll use placeholder values
        // This will need to be addressed when game-specific implementations are available
        let steering_pct = 0.0f32; // TODO: Extract from game-specific Moment implementation
        let yaw_rate = 0.0f32; // TODO: Extract from game-specific Moment implementation
        
        let yaw_rate_change = steering_pct.abs() - yaw_rate.abs();
        if steering_pct > MIN_STEERING_PCT_MEASURE
            && (brake >= MIN_BRAKE_PCT_MEASURE
                || throttle <= MAX_THROTTLE_PCT_MEASURE)
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
    use crate::telemetry::{SessionInfo, MockMoment, SerializableTelemetry, GameSource};

    #[test]
    fn test_no_scrub_annotation_due_to_insufficient_points() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = SerializableTelemetry {
            brake: Some(0.5),
            throttle: Some(0.3),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        // Add fewer samples than the minimum points threshold
        for _ in 0..4 {
            analyzer.analyze(&moment, &session_info);
        }

        let output = analyzer.analyze(&moment, &session_info);
        assert!(output.is_empty());
    }

    #[test]
    fn test_no_scrub_annotation_due_to_high_throttle() {
        let mut analyzer = ScrubAnalyzer::<10>::new(5);
        let telemetry_data = SerializableTelemetry {
            brake: Some(0.1),
            throttle: Some(0.5),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo::default();

        // Add enough samples to reach the minimum points threshold
        for _ in 0..5 {
            analyzer.analyze(&moment, &session_info);
        }

        let output = analyzer.analyze(&moment, &session_info);
        assert!(output.is_empty());
    }
    
    fn create_default_telemetry() -> SerializableTelemetry {
        SerializableTelemetry {
            point_no: 0,
            timestamp_ms: 0,
            game_source: GameSource::IRacing,
            gear: Some(1),
            speed_mps: Some(0.0),
            engine_rpm: Some(0.0),
            max_engine_rpm: Some(6000.0),
            shift_point_rpm: Some(5500.0),
            throttle: Some(0.0),
            brake: Some(0.0),
            clutch: Some(0.0),
            steering: None,
            steering_pct: None,
            lap_distance: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            abs_active: None,
            lat: None,
            lon: None,
            lat_accel: None,
            lon_accel: None,
            pitch: None,
            pitch_rate: None,
            roll: None,
            roll_rate: None,
            yaw: None,
            yaw_rate: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        }
    }
}
