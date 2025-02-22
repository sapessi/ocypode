use std::collections::HashMap;

use itertools::Itertools;
use simple_moving_average::{SumTreeSMA, SMA};

use super::{SessionInfo, TelemetryAnalyzer, TelemetryAnnotation, TelemetryPoint};

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
    fn analyze(
        &mut self,
        point: &TelemetryPoint,
        _: &SessionInfo,
    ) -> HashMap<String, TelemetryAnnotation> {
        // process expected RPM growth by gear
        let mut output: HashMap<String, TelemetryAnnotation> = HashMap::new();
        if point.cur_gear != self.prev_gear {
            self.prev_gear = point.cur_gear;
        } else {
            if point.cur_rpm > self.prev_rpm && point.cur_gear > 0 {
                let rpm_growth = point.cur_rpm - self.prev_rpm;

                if let Some(cur_average) = self.cur_averages.get(&point.cur_gear) {
                    if rpm_growth > *cur_average
                        && *self.cur_gear_points.entry(point.cur_gear).or_insert(0) >= WINDOW_SIZE
                    {
                        output.insert("wheelspin".to_string(), TelemetryAnnotation::Bool(true));
                        output.insert(
                            "rpm_change".to_string(),
                            TelemetryAnnotation::Float(rpm_growth),
                        );
                        output.insert(
                            "rpm_grpwth_avgs".to_string(),
                            TelemetryAnnotation::NumberMap(self.cur_averages.clone()),
                        );
                    }
                }
                // we only add a data point to our average if the user is in full acceleration
                if point.throttle > 0.95 && point.brake == 0. {
                    self.telemetry_window
                        .entry(point.cur_gear)
                        .or_insert_with(SumTreeSMA::new)
                        .add_sample(rpm_growth);

                    if *self.cur_gear_points.entry(point.cur_gear).or_insert(0) < WINDOW_SIZE {
                        self.cur_gear_points
                            .entry(point.cur_gear)
                            .and_modify(|e| *e += 1);
                    } else {
                        *self.cur_averages.entry(point.cur_gear).or_insert(0.) = *self
                            .telemetry_window
                            .get(&point.cur_gear)
                            .unwrap()
                            .get_sample_window_iter()
                            .sorted_by(|a, b| a.partial_cmp(b).unwrap())
                            .nth((WINDOW_SIZE as f32 * 0.9) as usize)
                            .unwrap();
                    }
                }
            }
            self.prev_rpm = point.cur_rpm;
        }

        output
    }
}
