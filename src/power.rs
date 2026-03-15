use crate::monitor::SystemMetrics;
use std::process::Command;

pub enum Governor {
    Performance,
    Powersave,
    Schedutil,
}

impl Governor {
    pub fn as_str(&self) -> &str {
        match self {
            Governor::Performance => "performance",
            Governor::Powersave => "powersave",
            Governor::Schedutil => "schedutil",
        }
    }
}

pub struct PowerManager;

impl PowerManager {
    pub fn new() -> Self {
        Self
    }

    pub fn apply_governor(&self, governor: Governor) -> Result<(), String> {
        let status = Command::new("cpufreqctl.auto-cpufreq")
            .arg("--governor")
            .arg(format!("--set={}", governor.as_str()))
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set governor".to_string())
        }
    }

    pub fn set_turbo(&self, enabled: bool) -> Result<(), String> {
        let val = if enabled { "0" } else { "1" }; // no_turbo: 0 means enabled
        let status = Command::new("cpufreqctl.auto-cpufreq")
            .arg("--no-turbo")
            .arg(format!("--set={}", val))
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set turbo mode".to_string())
        }
    }

    pub fn handle_state_change(&self, metrics: &SystemMetrics) {
        if let Some(true) = metrics.is_charging {
            let _ = self.apply_governor(Governor::Performance);
            let _ = self.set_turbo(true);
        } else {
            let _ = self.apply_governor(Governor::Powersave);
            // Disable turbo on battery unless load is very high
            if metrics.total_cpu_usage < 80.0 {
                let _ = self.set_turbo(false);
            }
        }
    }
}
