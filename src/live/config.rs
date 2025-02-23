use egui::Vec2;
use serde::{Deserialize, Serialize};

use crate::OcypodeError;

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
            Self::Vertical => Vec2::new(100., 500.),
            Self::Horizontal => Vec2::new(500., 100.),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AppConfig {
    pub(crate) refresh_rate_ms: usize,
    pub(crate) window_size_s: usize,
    pub(crate) show_alerts: bool,
    pub(crate) alerts_layout: AlertsLayout,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_rate_ms: REFRESH_RATE_MS,
            window_size_s: HISTORY_SECONDS,
            show_alerts: false,
            alerts_layout: AlertsLayout::Vertical,
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
