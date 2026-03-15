use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatteryStats {
    pub level: f32,
    pub is_charging: bool,
    pub start_threshold: Option<u8>,
    pub stop_threshold: Option<u8>,
    pub vendor: String,
}

pub trait BatteryProvider {
    fn get_stats(&self) -> Result<BatteryStats, String>;
    fn set_thresholds(&self, start: u8, stop: u8) -> Result<(), String>;
}

pub struct GenericLinuxBattery;

impl GenericLinuxBattery {
    pub fn new() -> Self {
        Self
    }

    fn read_sysfs(&self, path: &str) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }
}

impl BatteryProvider for GenericLinuxBattery {
    fn get_stats(&self) -> Result<BatteryStats, String> {
        // Basic Linux implementation via /sys/class/power_supply/BAT0/
        let bat_path = "/sys/class/power_supply/BAT0";
        if !Path::new(bat_path).exists() {
            return Err("No battery found at BAT0".to_string());
        }

        let level = self.read_sysfs(&format!("{}/capacity", bat_path))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);

        let status = self.read_sysfs(&format!("{}/status", bat_path))
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(BatteryStats {
            level,
            is_charging: status == "Charging",
            start_threshold: None, // Generic doesn't support thresholds
            stop_threshold: None,
            vendor: "Generic Linux".to_string(),
        })
    }

    fn set_thresholds(&self, _start: u8, _stop: u8) -> Result<(), String> {
        Err("Thresholds not supported by generic driver".to_string())
    }
}
