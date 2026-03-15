use crate::battery::{BatteryProvider, BatteryStats};
use std::fs;
use std::path::Path;

pub struct AsusBattery;

impl AsusBattery {
    pub fn new() -> Self {
        Self
    }

    fn read_sysfs(&self, path: &str) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn write_sysfs(&self, path: &str, value: &str) -> Result<(), String> {
        fs::write(path, value).map_err(|e| format!("Failed to write to {}: {}", path, e))
    }
}

impl BatteryProvider for AsusBattery {
    fn get_stats(&self) -> Result<BatteryStats, String> {
        let bat_path = "/sys/class/power_supply/BAT0";
        if !Path::new(bat_path).exists() {
            return Err("No battery found at BAT0".to_string());
        }

        let level = self.read_sysfs(&format!("{}/capacity", bat_path))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);

        let status = self.read_sysfs(&format!("{}/status", bat_path))
            .unwrap_or_else(|| "Unknown".to_string());

        let start = self.read_sysfs("/sys/class/power_supply/BAT0/charge_control_start_threshold")
            .and_then(|s| s.parse::<u8>().ok());
        
        let stop = self.read_sysfs("/sys/class/power_supply/BAT0/charge_control_end_threshold")
            .and_then(|s| s.parse::<u8>().ok());

        Ok(BatteryStats {
            level,
            is_charging: status == "Charging",
            start_threshold: start,
            stop_threshold: stop,
            vendor: "Asus".to_string(),
        })
    }

    fn set_thresholds(&self, start: u8, stop: u8) -> Result<(), String> {
        // Asus specific safety: stop usually must be > start
        self.write_sysfs("/sys/class/power_supply/BAT0/charge_control_end_threshold", "100")?;
        self.write_sysfs("/sys/class/power_supply/BAT0/charge_control_start_threshold", &start.to_string())?;
        self.write_sysfs("/sys/class/power_supply/BAT0/charge_control_end_threshold", &stop.to_string())?;
        Ok(())
    }
}
