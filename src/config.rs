use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub operation_mode: String, // "auto", "performance", "efficiency"
    pub battery_threshold: u8,
    pub usb_autosuspend: bool,
    pub sata_alpm: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            operation_mode: "auto".to_string(),
            battery_threshold: 80,
            usb_autosuspend: true, // Safe default for Linux
            sata_alpm: true,       // Safe default for Linux
        }
    }
}

impl AppConfig {
    fn get_path() -> PathBuf {
        let mut path = PathBuf::from("/etc/zenith-energy");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        if let Ok(content) = fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_path();
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }
}