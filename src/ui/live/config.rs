use egui::{Pos2, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::OcypodeError;
use crate::setup_assistant::{Finding, FindingType};

use super::{HISTORY_SECONDS, REFRESH_RATE_MS};

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum AlertsLayout {
    Vertical,
    Horizontal,
}

impl AlertsLayout {
    pub(crate) fn window_size(&self) -> Vec2 {
        match self {
            Self::Vertical => Vec2::new(70., 500.),
            Self::Horizontal => Vec2::new(360., 100.),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct WindowPosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Default for WindowPosition {
    fn default() -> Self {
        Self { x: 0., y: 0. }
    }
}

impl From<WindowPosition> for Pos2 {
    fn from(value: WindowPosition) -> Self {
        Pos2::new(value.x, value.y)
    }
}

impl From<Pos2> for WindowPosition {
    fn from(value: Pos2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub(crate) struct AppConfig {
    pub(crate) refresh_rate_ms: usize,
    pub(crate) window_size_s: usize,
    pub(crate) show_alerts: bool,
    pub(crate) alerts_layout: AlertsLayout,
    pub(crate) telemetry_window_position: WindowPosition,
    pub(crate) alert_window_position: WindowPosition,
    pub(crate) show_setup_window: bool,
    pub(crate) setup_window_position: WindowPosition,
    pub(crate) setup_assistant_findings: HashMap<FindingType, Finding>,
    pub(crate) setup_assistant_confirmed_findings: HashSet<FindingType>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_rate_ms: REFRESH_RATE_MS,
            window_size_s: HISTORY_SECONDS,
            show_alerts: false,
            alerts_layout: AlertsLayout::Vertical,
            telemetry_window_position: WindowPosition::default(),
            alert_window_position: WindowPosition::default(),
            show_setup_window: false,
            setup_window_position: WindowPosition::default(),
            setup_assistant_findings: HashMap::new(),
            setup_assistant_confirmed_findings: HashSet::new(),
        }
    }
}

impl AppConfig {
    pub(crate) fn from_local_file() -> Option<Self> {
        let config_path = dirs::config_dir()?.join("ocypode").join(CONFIG_FILE_NAME);

        if config_path.exists() {
            let file = std::fs::File::open(config_path).expect("Could not open config file");
            Some(serde_json::from_reader(file).expect("Could not parse config file"))
        } else {
            None
        }
    }

    pub(crate) fn save(&self) -> Result<(), OcypodeError> {
        let config_path = dirs::config_dir()
            .ok_or(OcypodeError::NoConfigDir)?
            .join("ocypode")
            .join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            std::fs::create_dir_all(config_path.parent().unwrap())
                .map_err(|e| OcypodeError::ConfigIOError { source: e })?;
        }

        let file = std::fs::File::create(config_path)
            .map_err(|e| OcypodeError::ConfigIOError { source: e })?;
        serde_json::to_writer(file, self)
            .map_err(|e| OcypodeError::ConfigSerializeError { source: e })
    }
}
