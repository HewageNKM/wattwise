use crate::battery::{BatteryProvider, BatteryStats};
use std::fs;
use std::path::Path;

pub struct IdeaPadBattery;

const CONSERVATION_MODE_PATH: &str = "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00/conservation_mode";

impl IdeaPadBattery {
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

impl BatteryProvider for IdeaPadBattery {
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

        let conservation = self.read_sysfs(CONSERVATION_MODE_PATH)
            .map(|s| s == "1")
            .unwrap_or(false);

        Ok(BatteryStats {
            level,
            is_charging: status == "Charging",
            // IdeaPad uses toggles, not specific % thresholds
            start_threshold: None,
            stop_threshold: if conservation { Some(60) } else { Some(100) },
            vendor: "IdeaPad".to_string(),
        })
    }

    fn set_thresholds(&self, _start: u8, stop: u8) -> Result<(), String> {
        // IdeaPad conservation mode is typically 60%
        let value = if stop <= 60 { "1" } else { "0" };
        self.write_sysfs(CONSERVATION_MODE_PATH, value)
    }
}
