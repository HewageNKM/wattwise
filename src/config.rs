use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub operation_mode: String, // "auto", "performance", "efficiency"
    pub usb_autosuspend: bool,
    pub sata_alpm: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            operation_mode: "auto".to_string(),
            usb_autosuspend: true, // Safe default for Linux
            sata_alpm: true,       // Safe default for Linux
        }
    }
}

impl AppConfig {
    fn get_path() -> PathBuf {
        // 1. Try to find the real user's home even if we are running as root
        let home = if let Ok(h) = std::env::var("HOME") {
            if h == "/root" {
                // We are root, try to find the standard user's home
                if let Ok(entries) = fs::read_dir("/home") {
                    entries.flatten()
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.path().to_string_lossy().to_string())
                        .next()
                        .unwrap_or_else(|| "/etc/wattwise".to_string())
                } else {
                    "/etc/wattwise".to_string()
                }
            } else {
                h
            }
        } else {
            "/etc/wattwise".to_string()
        };

        let mut path = PathBuf::from(home);
        if path.starts_with("/home") {
            path.push(".config/wattwise");
        }
        
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