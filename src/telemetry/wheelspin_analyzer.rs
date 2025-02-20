use std::collections::HashMap;

use simple_moving_average::{SumTreeSMA, SMA};

use super::{SessionInfo, TelemetryAnalyzer, TelemetryAnnotation, TelemetryPoint};

pub struct WheelspinAnalyzer<const WINDOW_SIZE: usize> {
    cur_averages: HashMap<u32, f32>,
    telemetry_window: HashMap<u32, SumTreeSMA<f32, f32, WINDOW_SIZE>>,
    prev_gear: u32,
    prev_rpm: f32,
    cur_wheelspin_points: usize,
}

impl<const WINDOW_SIZE: usize> WheelspinAnalyzer<WINDOW_SIZE> {
    pub fn new() -> Self {
        Self {
            cur_averages: HashMap::new(),
            telemetry_window: HashMap::new(),
            prev_gear: 0,
            prev_rpm: 0.,
            cur_wheelspin_points: 0,
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
                    if rpm_growth > *cur_average * 1.1 && self.cur_wheelspin_points >= WINDOW_SIZE {
                        output.insert("wheelspin".to_string(), TelemetryAnnotation::Bool(true));
                    }
                }
                // we only add a data point to our average if the user is in full acceleration
                if point.throttle > 0.8 {
                    self.telemetry_window
                        .entry(point.cur_gear)
                        .or_insert_with(SumTreeSMA::new)
                        .add_sample(rpm_growth);
                    *self.cur_averages.entry(point.cur_gear).or_insert(0.) = self
                        .telemetry_window
                        .get(&point.cur_gear)
                        .unwrap()
                        .get_average();
                    if self.cur_wheelspin_points < WINDOW_SIZE {
                        self.cur_wheelspin_points += 1;
                    } else {
                        output.insert(
                            "rpm_grpwth_avgs".to_string(),
                            TelemetryAnnotation::NumberMap(self.cur_averages.clone()),
                        );
                    }
                }
            }
            self.prev_rpm = point.cur_rpm;
        }

        output
    }
}
