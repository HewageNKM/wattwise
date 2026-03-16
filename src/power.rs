use crate::monitor::SystemMetrics;
use crate::config::AppConfig;
use crate::battery;
use std::process::Command;
use std::fs;
use std::path::Path;

pub enum Governor {
    Performance,
    Powersave,
    Schedutil,
}

pub enum EnergyPreference {
    Performance,
    BalancePerformance,
    BalancePower,
    Power,
}

impl EnergyPreference {
    pub fn as_str(&self) -> &str {
        match self {
            EnergyPreference::Performance => "performance",
            EnergyPreference::BalancePerformance => "balance_performance",
            EnergyPreference::BalancePower => "balance_power",
            EnergyPreference::Power => "power",
        }
    }
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

pub struct PowerManager {}

impl PowerManager {
    pub fn new() -> Self {
        Self {}
    }

    fn set_core_online(&self, id: usize, online: bool) -> Result<(), String> {
        if id == 0 { return Ok(()); } 
        let path = format!("/sys/devices/system/cpu/cpu{}/online", id);
        let val = if online { "1" } else { "0" };
        if std::path::Path::new(&path).exists() {
            self.write_sysfs(&path, val)
        } else {
            Ok(())
        }
    }

    pub fn park_cores(&self, max_online: Option<usize>) {
        let cpu_dir = "/sys/devices/system/cpu";
        if let Ok(entries) = std::fs::read_dir(cpu_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(id) = name[3..].parse::<usize>() {
                        if id > 0 {
                            let online = match max_online {
                                Some(m) => id < m,
                                None => true, // unpark all
                            };
                            let _ = self.set_core_online(id, online);
                        }
                    }
                }
            }
        }
    }

    fn get_process_category(&self, name: &str) -> &'static str {
        let n = name.to_lowercase();
        if n.contains("steam") || n.contains("csgo") || n.contains("dota") || n.contains("cyberpunk") || n.contains("hl2") {
            "gaming"
        } else if n.contains("node") || n.contains("vscode") || n.contains("cargo") || n.contains("rustc") {
            "development"
        } else if n.contains("firefox") || n.contains("chrome") || n.contains("brave") {
            "browsing"
        } else {
            "general"
        }
    }

    pub fn set_usb_autosuspend(&self, enabled: bool) {
        let val = if enabled { "auto" } else { "on" };
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("for d in /sys/bus/usb/devices/*/power/control; do echo {} > \"$d\" 2>/dev/null; done", val))
            .status();
    }

    pub fn set_sata_alpm(&self, enabled: bool) {
        let val = if enabled { "med_power_with_dipm" } else { "max_performance" };
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("for d in /sys/class/scsi_host/host*/link_power_management_policy; do echo {} > \"$d\" 2>/dev/null; done", val))
            .status();
    }

    fn write_sysfs(&self, path: &str, value: &str) -> Result<(), String> {
        fs::write(path, value).map_err(|e| format!("Failed to write to {}: {}", path, e))
    }

    fn write_to_all_cpus(&self, subpath: &str, value: &str) -> Result<(), String> {
        let mut errors = Vec::new();
        let cpu_dir = "/sys/devices/system/cpu";
        
        if let Ok(entries) = fs::read_dir(cpu_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                    let full_path = format!("{}/{}/{}", cpu_dir, name, subpath);
                    if Path::new(&full_path).exists() {
                        if let Err(e) = self.write_sysfs(&full_path, value) {
                            errors.push(format!("{}: {}", name, e));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    pub fn apply_governor(&self, governor: Governor) -> Result<(), String> {
        let val = governor.as_str();
        // Try native first
        if self.write_to_all_cpus("cpufreq/scaling_governor", val).is_ok() {
            return Ok(());
        }

        // Fallback to cpufreqctl
        let status = Command::new("zenith-ctl")
            .arg("--governor")
            .arg(format!("--set={}", val))
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set governor".to_string())
        }
    }

    pub fn apply_epp(&self, preference: EnergyPreference) -> Result<(), String> {
        let val = preference.as_str();
        // Try native first
        if self.write_to_all_cpus("cpufreq/energy_performance_preference", val).is_ok() {
            return Ok(());
        }

        let status = Command::new("zenith-ctl")
            .arg("--epp")
            .arg(format!("--set={}", val))
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set EPP".to_string())
        }
    }

    pub fn apply_epb(&self, value: u8) -> Result<(), String> {
        let val = value.to_string();
        // Try native first (often at /sys/devices/system/cpu/cpu*/power/energy_perf_bias)
        if self.write_to_all_cpus("power/energy_perf_bias", &val).is_ok() {
            return Ok(());
        }

        let status = Command::new("zenith-ctl")
            .arg("--epb")
            .arg(format!("--set={}", value))
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set EPB".to_string())
        }
    }

    pub fn set_turbo(&self, enabled: bool) -> Result<(), String> {
        // Try native Intel first
        let intel_path = "/sys/devices/system/cpu/intel_pstate/no_turbo";
        let turbo_val = if enabled { "0" } else { "1" };
        if Path::new(intel_path).exists() {
            if self.write_sysfs(intel_path, turbo_val).is_ok() {
                return Ok(());
            }
        }

        // Try native AMD/Other
        let boost_path = "/sys/devices/system/cpu/cpufreq/boost";
        let boost_val = if enabled { "1" } else { "0" };
        if Path::new(boost_path).exists() {
            if self.write_sysfs(boost_path, boost_val).is_ok() {
                return Ok(());
            }
        }

        // Fallback
        let cmd_val = if enabled { "0" } else { "1" };
        let status = Command::new("zenith-ctl")
            .arg("--no-turbo")
            .arg(format!("--set={}", cmd_val))
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to set turbo mode".to_string())
        }
    }

    pub fn handle_state_change(&self, metrics: &SystemMetrics) -> std::time::Duration {
        let config = AppConfig::load();
        
        // AI Proactive Ruleset: Detect active process category
        let mut is_gaming = false;
        if let Some(proc) = metrics.top_processes.first() {
            if self.get_process_category(&proc.name) == "gaming" {
                is_gaming = true;
            }
        }

        let is_high_load = metrics.total_cpu_usage > 40.0;
        let battery_level = metrics.battery_level.unwrap_or(100.0);

        // Always ensure battery threshold is set according to config
        let b = battery::get_vendor_battery();
        let _ = b.set_thresholds(0, config.battery_threshold);

        // 🟢 1. Determine Base Profile State
        let profile = if let Some(ov) = &config.manual_override {
            if ov == "performance" { config.ac_profile.clone() } else { config.bat_profile.clone() }
        } else if metrics.is_charging.unwrap_or(false) {
            config.ac_profile.clone()
        } else {
            config.bat_profile.clone()
        };

        // 🟠 2. Apply Base Profile Parameters
        let _ = self.apply_governor_str(&profile.governor);
        let mut turbo = profile.turbo;
        
        if profile.core_parking {
            self.park_cores(Some(2)); 
        } else {
            self.park_cores(None); 
        }

        self.set_usb_autosuspend(profile.usb_autosuspend);
        self.set_sata_alpm(profile.sata_alpm);

        // 🔴 3. Dynamic Automated Overlays (Safeguards & Autopilot Intelligence)
        if !metrics.is_charging.unwrap_or(false) {
            // Emergency Power Saving below 15%
            if battery_level <= 15.0 {
                self.park_cores(Some(2));
                let _ = self.apply_governor_str("powersave");
                let _ = self.apply_epp(EnergyPreference::Power);
                let _ = self.apply_epb(15);
                turbo = false;
            } else if battery_level <= 20.0 {
                let _ = self.apply_governor_str("powersave");
                let _ = self.apply_epp(EnergyPreference::BalancePower);
                let _ = self.apply_epb(10);
            } else if is_high_load && config.manual_override.is_none() {
                // ⚡ Anti-Lag Burst Lift (Autopilot Intelligence Mode ONLY)
                // Lift caps temporarily for high demands avoiding battery stutter
                let _ = self.apply_governor_str("performance");
                self.park_cores(None);
                turbo = true;
            }

            if is_gaming && metrics.total_cpu_usage > 70.0 {
                turbo = false; 
            }
        } else {
             if config.manual_override.is_none() {
                 if is_high_load {
                     let _ = self.apply_governor_str("performance");
                     turbo = true;
                 } else {
                     let _ = self.apply_governor_str("powersave");
                     turbo = false; 
                 }
             }

             // Charge State Tuning overlays
             if profile.governor == "performance" {
                 let _ = self.apply_epp(EnergyPreference::Performance);
                 let _ = self.apply_epb(0);
             }
        }

        let _ = self.set_turbo(turbo);

        // 🔄 4. Return Adaptive Polling Cycle Time
        if is_high_load || is_gaming {
            std::time::Duration::from_secs(1)
        } else {
            std::time::Duration::from_secs(5)
        }
    }

    pub fn apply_governor_str(&self, gov: &str) -> Result<(), String> {
        let g = match gov {
            "performance" => Governor::Performance,
            "powersave" => Governor::Powersave,
            "schedutil" => Governor::Schedutil,
            _ => return Err("Invalid governor".to_string()),
        };
        self.apply_governor(g)
    }
}
