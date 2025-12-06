use super::{SessionInfo, TelemetryAnalyzer, TelemetryData};

pub(crate) const MIN_TRAILBRAKING_PCT: f32 = 0.2;
pub(crate) const MAX_TRAILBRAKING_STEERING_ANGLE: f32 = 0.1;

pub struct TrailbrakeSteeringAnalyzer {
    max_trailbraking_steering_angle: f32,
    min_trailbraking_pct: f32,
}

impl TrailbrakeSteeringAnalyzer {
    pub fn new(max_trailbraking_steering_angle: f32, min_trailbraking_pct: f32) -> Self {
        Self {
            max_trailbraking_steering_angle,
            min_trailbraking_pct,
        }
    }
}

impl TelemetryAnalyzer for TrailbrakeSteeringAnalyzer {
    fn analyze(
        &mut self,
        telemetry: &TelemetryData,
        session_info: &SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();

        // Extract brake and steering data from TelemetryData
        let brake = telemetry.brake.unwrap_or(0.0);
        let steering_angle_rad = telemetry.steering_angle_rad.unwrap_or(0.0);
        let steering_pct = telemetry.steering_pct.unwrap_or(0.0);

        // nothing to process here if we cannot establish the current steering pct
        if session_info.max_steering_angle == 0. {
            return output;
        }
        // this should not be possible
        if steering_angle_rad > session_info.max_steering_angle {
            return output;
        }

        // we are braking... measure steering angle
        if brake > self.min_trailbraking_pct && steering_pct > self.max_trailbraking_steering_angle
        {
            output.push(super::TelemetryAnnotation::TrailbrakeSteering {
                cur_trailbrake_steering: steering_pct,
                is_excessive_trailbrake_steering: true,
            });
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use crate::telemetry::{GameSource, TelemetryData};

    use super::*;

    fn default_analyzer() -> TrailbrakeSteeringAnalyzer {
        TrailbrakeSteeringAnalyzer::new(0.1, 0.2)
    }

    #[test]
    fn test_fails_if_no_max_steering() {
        let mut analyzer = default_analyzer();
        let telemetry_data = create_default_telemetry();
        let annotations = analyzer.analyze(&telemetry_data, &SessionInfo::default());
        assert!(annotations.is_empty());
    }

    #[test]
    fn test_doesnt_fire_with_low_brake() {
        let mut analyzer = default_analyzer();
        let telemetry_data = TelemetryData {
            brake: Some(0.1),
            ..create_default_telemetry()
        };
        let session_info = SessionInfo {
            max_steering_angle: 0.5,
            ..Default::default()
        };
        assert!(analyzer.analyze(&telemetry_data, &session_info).is_empty());
    }

    fn create_default_telemetry() -> TelemetryData {
        TelemetryData {
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
            steering_angle_rad: None,
            steering_pct: None,
            lap_distance_m: None,
            lap_distance_pct: None,
            lap_number: None,
            last_lap_time_s: None,
            best_lap_time_s: None,
            is_pit_limiter_engaged: None,
            is_in_pit_lane: None,
            is_abs_active: None,
            latitude_deg: None,
            longitude_deg: None,
            lateral_accel_mps2: None,
            longitudinal_accel_mps2: None,
            pitch_rad: None,
            pitch_rate_rps: None,
            roll_rad: None,
            roll_rate_rps: None,
            yaw_rad: None,
            yaw_rate_rps: None,
            lf_tire_info: None,
            rf_tire_info: None,
            lr_tire_info: None,
            rr_tire_info: None,
            annotations: Vec::new(),
        }
    }
}
