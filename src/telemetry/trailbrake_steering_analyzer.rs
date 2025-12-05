use super::{SessionInfo, TelemetryAnalyzer};
use simetry::Moment;

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
        telemetry: &dyn Moment,
        session_info: &SessionInfo,
    ) -> Vec<super::TelemetryAnnotation> {
        let mut output = Vec::new();
        
        // Extract data from Moment trait
        let pedals = telemetry.pedals();
        let brake = pedals.as_ref().map(|p| p.brake as f32).unwrap_or(0.0);
        
        // Note: steering and steering_pct are not available in the base Moment trait
        // For now, we'll use placeholder values
        // This will need to be addressed when game-specific implementations are available
        let steering = 0.0f32; // TODO: Extract from game-specific Moment implementation
        let steering_pct = 0.0f32; // TODO: Extract from game-specific Moment implementation
        
        // nothing to process here if we cannot establish the current steering pct
        if session_info.max_steering_angle == 0. {
            return output;
        }
        // this should not be possible
        if steering > session_info.max_steering_angle {
            return output;
        }

        // we are braking... measure steering angle
        if brake > self.min_trailbraking_pct
            && steering_pct > self.max_trailbraking_steering_angle
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
    use crate::telemetry::{MockMoment, SerializableTelemetry, GameSource};

    use super::*;

    fn default_analyzer() -> TrailbrakeSteeringAnalyzer {
        TrailbrakeSteeringAnalyzer::new(0.1, 0.2)
    }

    #[test]
    fn test_fails_if_no_max_steering() {
        let mut analyzer = default_analyzer();
        let telemetry_data = create_default_telemetry();
        let moment = MockMoment::new(telemetry_data);
        let annotations = analyzer.analyze(&moment, &SessionInfo::default());
        assert!(annotations.is_empty());
    }

    #[test]
    fn test_doesnt_fire_with_low_brake() {
        let mut analyzer = default_analyzer();
        let telemetry_data = SerializableTelemetry {
            brake: Some(0.1),
            ..create_default_telemetry()
        };
        let moment = MockMoment::new(telemetry_data);
        let session_info = SessionInfo {
            max_steering_angle: 0.5,
            ..Default::default()
        };
        assert!(analyzer.analyze(&moment, &session_info).is_empty());
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
