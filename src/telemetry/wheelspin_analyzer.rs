use std::collections::HashMap;

use itertools::Itertools;
use simple_moving_average::{SMA, SumTreeSMA};
use simetry::Moment;
use uom::si::angular_velocity::revolution_per_minute;

use super::{SessionInfo, TelemetryAnalyzer, TelemetryAnnotation};

pub struct WheelspinAnalyzer<const WINDOW_SIZE: usize> {
    cur_averages: HashMap<u32, f32>,
    telemetry_window: HashMap<u32, SumTreeSMA<f32, f32, WINDOW_SIZE>>,
    prev_gear: u32,
    prev_rpm: f32,
    cur_gear_points: HashMap<u32, usize>,
}

impl<const WINDOW_SIZE: usize> WheelspinAnalyzer<WINDOW_SIZE> {
    pub fn new() -> Self {
        Self {
            cur_averages: HashMap::new(),
            telemetry_window: HashMap::new(),
            prev_gear: 0,
            prev_rpm: 0.,
            cur_gear_points: HashMap::new(),
        }
    }
}

impl<const WINDOW_SIZE: usize> TelemetryAnalyzer for WheelspinAnalyzer<WINDOW_SIZE> {
    fn analyze(&mut self, telemetry: &dyn Moment, _: &SessionInfo) -> Vec<TelemetryAnnotation> {
        // process expected RPM growth by gear
        let mut output = Vec::new();
        
        // Extract data from Moment trait
        let cur_gear = telemetry.vehicle_gear().unwrap_or(0).max(0) as u32;
        let cur_rpm = telemetry.vehicle_engine_rotation_speed()
            .map(|rpm| rpm.get::<revolution_per_minute>() as f32)
            .unwrap_or(0.0);
        let pedals = telemetry.pedals();
        let throttle = pedals.as_ref().map(|p| p.throttle as f32).unwrap_or(0.0);
        let brake = pedals.as_ref().map(|p| p.brake as f32).unwrap_or(0.0);
        
        if cur_gear != self.prev_gear {
            self.prev_gear = cur_gear;
        } else {
            if cur_rpm > self.prev_rpm && cur_gear > 0 {
                let rpm_growth = cur_rpm - self.prev_rpm;

                if let Some(cur_average) = self.cur_averages.get(&cur_gear) {
                    if rpm_growth > *cur_average
                        && *self.cur_gear_points.entry(cur_gear).or_insert(0) >= WINDOW_SIZE
                    {
                        output.push(TelemetryAnnotation::Wheelspin {
                            avg_rpm_increase_per_gear: self.cur_averages.clone(),
                            cur_gear,
                            cur_rpm_increase: rpm_growth,
                            is_wheelspin: true,
                        });
                    }
                }
                // we only add a data point to our average if the user is in full acceleration
                if throttle > 0.95 && brake == 0. {
                    self.telemetry_window
                        .entry(cur_gear)
                        .or_insert_with(SumTreeSMA::new)
                        .add_sample(rpm_growth);

                    if *self.cur_gear_points.entry(cur_gear).or_insert(0) < WINDOW_SIZE {
                        self.cur_gear_points
                            .entry(cur_gear)
                            .and_modify(|e| *e += 1);
                    } else {
                        *self.cur_averages.entry(cur_gear).or_insert(0.) = *self
                            .telemetry_window
                            .get(&cur_gear)
                            .unwrap()
                            .get_sample_window_iter()
                            .sorted_by(|a, b| a.partial_cmp(b).unwrap())
                            .nth((WINDOW_SIZE as f32 * 0.9) as usize)
                            .unwrap();
                    }
                }
            }
            self.prev_rpm = cur_rpm;
        }

        output
    }
}
