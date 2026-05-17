use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub port: u16,
    pub use_mph: bool,
    pub tire_temp_cold: f32,
    pub tire_temp_optimal: f32,
    pub tire_temp_hot: f32,
    pub auto_record: bool,
    #[serde(default = "Settings::default_theme")]
    pub theme: String,
}

impl Settings {
    fn default_theme() -> String { "dark".to_string() }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: 20440,
            use_mph: true,
            tire_temp_cold: 60.0,
            tire_temp_optimal: 85.0,
            tire_temp_hot: 110.0,
            auto_record: true,
            theme: Self::default_theme(),
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

fn settings_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fh6-tel")
        .join("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> Result<(), SettingsError> {
    let path = settings_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_string_pretty(s)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_is_20440() {
        let s = Settings::default();
        assert_eq!(s.port, 20440);
    }

    #[test]
    fn roundtrip_to_json() {
        let s = Settings { port: 9999, use_mph: false, ..Settings::default() };
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s2.port, 9999);
        assert!(!s2.use_mph);
    }
}
