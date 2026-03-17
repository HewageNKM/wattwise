use crate::monitor::SystemMetrics;
use crate::config::AppConfig;
use crate::battery;
use std::process::Command;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tier {
    Eco,
    Balanced,
    Performance,
    Extreme,
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

pub struct PowerManager {
    prev_cpu_usage: AtomicU32,
    rolling_load_avg: AtomicU32,
    total_cores: usize,
    
    current_mode: Mutex<String>,
    current_usb_autosuspend: AtomicBool,
    current_sata_alpm: AtomicBool,
    current_unpark_count: AtomicUsize,
    current_turbo_failed: AtomicBool,
    current_stage: AtomicUsize, // Tracks the current step on the staircase
}

impl PowerManager {
    pub fn new() -> Self {
        // Explicitly define as usize to fix the compiler type ambiguity
        let total_cores: usize = {
            let mut count = 0;
            if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                        count += 1;
                    }
                }
            }
            if count == 0 { 4 } else { count }
        };

        // Calculate the maximum possible stage based on total cores
        let max_stage = (total_cores.saturating_sub(2)) * 2 + 1;

        Self {
            prev_cpu_usage: AtomicU32::new(0_f32.to_bits()),
            rolling_load_avg: AtomicU32::new(0_f32.to_bits()),
            total_cores,
            current_mode: Mutex::new("auto".to_string()),
            current_usb_autosuspend: AtomicBool::new(false),
            current_sata_alpm: AtomicBool::new(false),
            current_unpark_count: AtomicUsize::new(total_cores), 
            current_turbo_failed: AtomicBool::new(false),
            current_stage: AtomicUsize::new(max_stage), // Start at max power, scale down gracefully
        }
    }

    // SIMPLIFIED LOGGING: Systemd handles routing this to /var/log/zenith-energy.log automatically
    fn log_to_file(&self, message: &str) {
        println!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), message);
    }

    fn set_core_online(&self, id: usize, online: bool) -> Result<(), String> {
        if id == 0 { return Ok(()); } // Never offline core 0
        let path = format!("/sys/devices/system/cpu/cpu{}/online", id);
        let val = if online { "1" } else { "0" };
        
        if std::path::Path::new(&path).exists() {
            match self.write_sysfs_smart(&path, val) {
                Ok(true) => {
                    self.log_to_file(&format!("✅ Core {} set online={}", id, online));
                    Ok(())
                },
                Ok(false) => Ok(()), 
                Err(_) => {
                    let status = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(format!("echo {} | tee \"{}\" > /dev/null", val, path))
                        .status();
                    
                    if status.is_ok() && status.unwrap().success() {
                        self.log_to_file(&format!("✅ Core {} set online={} (via tee)", id, online));
                        Ok(())
                    } else {
                        let err = format!("Failed to write to {} (direct & tee)", path);
                        self.log_to_file(&format!("❌ {}", err));
                        Err(err)
                    }
                }
            }
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
                                None => true, 
                            };
                            if let Err(e) = self.set_core_online(id, online) {
                                self.log_to_file(&format!("❌ Core Parking Failed (Core {} online={}): {}", id, online, e));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_usb_autosuspend(&self, enabled: bool) {
        if self.current_usb_autosuspend.swap(enabled, Ordering::SeqCst) == enabled {
            return; 
        }
        let val = if enabled { "auto" } else { "on" };
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("for d in /sys/bus/usb/devices/*/power/control; do echo {} > \"$d\" 2>/dev/null; done", val))
            .status();
        self.log_to_file(&format!("🔌 USB Autosuspend: {}", if enabled { "Enabled (auto)" } else { "Disabled (on)" }));
    }

    pub fn set_sata_alpm(&self, enabled: bool) {
        if self.current_sata_alpm.swap(enabled, Ordering::SeqCst) == enabled {
            return; 
        }
        let val = if enabled { "med_power_with_dipm" } else { "max_performance" };
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("for d in /sys/class/scsi_host/host*/link_power_management_policy; do [ -f \"$d\" ] && echo {} > \"$d\" 2>/dev/null; done", val))
            .status();
        self.log_to_file(&format!("💽 SATA ALPM: {}", if enabled { "Enabled (med_power)" } else { "Disabled (max_perf)" }));
    }

    fn write_sysfs(&self, path: &str, value: &str) -> Result<(), String> {
        fs::write(path, value).map_err(|e| format!("Failed to write to {}: {}", path, e))
    }

    fn write_sysfs_smart(&self, path: &str, value: &str) -> Result<bool, String> {
        if let Ok(current) = fs::read_to_string(path) {
            if current.trim() == value {
                return Ok(false); 
            }
        }
        self.write_sysfs(path, value).map(|_| true)
    }

    pub fn apply_epp(&self, preference: EnergyPreference) -> Result<(), String> {
        let val = preference.as_str();
        // 100% Native Rust. Quietly ignore "Device or resource busy" on sleeping cores.
        let _ = self.write_to_all_cpus("cpufreq/energy_performance_preference", val);
        Ok(())
    }

    pub fn apply_epb(&self, value: u8) -> Result<(), String> {
        let val = value.to_string();
        // 100% Native Rust. Quietly ignore "Device or resource busy" on sleeping cores.
        let _ = self.write_to_all_cpus("power/energy_perf_bias", &val);
        Ok(())
    }

    pub fn set_turbo(&self, enabled: bool) -> Result<(), String> {
        let paths = [
            "/sys/devices/system/cpu/intel_pstate/no_turbo",
            "/sys/devices/system/cpu/cpufreq/boost"
        ];
        
        for path in paths {
            if std::path::Path::new(path).exists() {
                if self.current_turbo_failed.load(Ordering::Relaxed) {
                    continue; 
                }

                let val = if path.contains("boost") {
                    if enabled { "1" } else { "0" }
                } else {
                    if enabled { "0" } else { "1" }
                };
                
                match self.write_sysfs_smart(path, val) {
                    Ok(true) => self.log_to_file(&format!("✅ Turbo Boost set to {} ({})", enabled, path)),
                    Ok(false) => {}, 
                    Err(e) => {
                        self.log_to_file(&format!("❌ Failed to set Turbo Boost ({:?}): {}", path, e));
                        if e.contains("Permission denied") {
                            self.current_turbo_failed.store(true, Ordering::Relaxed);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The Pure EMA Staircase Engine
    fn calculate_fluid_state(&self, load: f32, is_charging: bool, battery_level: f32) -> (usize, u32) {
        let ratio = load / 100.0;
        let mut cores = (self.total_cores as f32 * ratio).ceil() as usize;
        
        if !is_charging && battery_level < 50.0 {
            let battery_factor = battery_level / 50.0; 
            cores = (cores as f32 * battery_factor).ceil() as usize;
        }
        
        let unpark_count = cores.clamp(2, self.total_cores);
        let base_ceiling = 35.0; 
        let scale_factor = 1.6; 
        
        let mut ceiling = base_ceiling + (load / scale_factor); 
        if ceiling.is_nan() {
            ceiling = base_ceiling;
        }
        let max_perf_pct = ceiling.clamp(base_ceiling, 100.0) as u32;

        (unpark_count, max_perf_pct)
    }

    pub fn handle_state_change(&self, metrics: &SystemMetrics) -> std::time::Duration {
        let config = AppConfig::load();
        let battery_level = metrics.battery_level.unwrap_or(100.0);
        let is_charging = metrics.is_charging.unwrap_or(false);
        
        // 1. Calculate Acceleration Derivative & EWMA
        let current_load = metrics.total_cpu_usage;
        let prev_bits = self.prev_cpu_usage.swap(current_load.to_bits(), Ordering::Relaxed);
        let accel = current_load - f32::from_bits(prev_bits);

        let alpha = 0.12_f32; 
        let prev_avg = f32::from_bits(self.rolling_load_avg.load(Ordering::Relaxed));
        let rolling_avg = (alpha * current_load) + ((1.0 - alpha) * prev_avg);
        self.rolling_load_avg.store(rolling_avg.to_bits(), Ordering::Relaxed);

        // Adaptive Thresholds
        let mut eco_thresh = 15.0;
        let mut bal_thresh = 50.0;

        // 2. Determine Continuous Curve Tier (Symmetrical)
        let mut tier = match rolling_avg {
            l if l < eco_thresh && accel < 3.0 => Tier::Eco,
            l if l < bal_thresh => Tier::Balanced,
            l if l < 75.0 => Tier::Performance,
            _ => Tier::Extreme,
        };

        // 🌡️ 3. Thermal & Safety Guards
        let cpu_temp = metrics.cpu_temperature.unwrap_or(0.0);
        if cpu_temp > 85.0 {
            tier = Tier::Eco;
            self.log_to_file(&format!("⚠️ Thermal Throttle: {:.1}°C - Forcing Eco", cpu_temp));
        } else if !is_charging && battery_level <= 15.0 {
            tier = Tier::Eco;
        }

        self.log_to_file(&format!("Autopilot: Load={:.1}%, Accel={:.1}, Rolling={:.1}% | Tier={:?}", 
            current_load, accel, rolling_avg, tier));

        // 5. Continuous Scaling State
        let (unpark_count, max_perf_pct) = self.calculate_fluid_state(rolling_avg, is_charging, battery_level);
        let perf_str = max_perf_pct.to_string();
        let _ = self.write_sysfs_smart("/sys/devices/system/cpu/intel_pstate/max_perf_pct", &perf_str);
        
        let turbo = max_perf_pct >= 88 && tier != Tier::Eco;
        let _ = self.set_turbo(turbo);

        // Write Shared State Frame Node triggers
        let state_json = format!(
            "{{\"unpark_count\": {}, \"max_perf_pct\": {}, \"tier\": \"{:?}\"}}",
            unpark_count, max_perf_pct, tier
        );
        let _ = std::fs::write("/run/zenith-energy.state", state_json);

        // 6. Apply Autopilot Hardware States
        let mut current_mode_lock = self.current_mode.lock().unwrap();
        let active_mode = match tier {
            Tier::Eco => "eco",
            Tier::Balanced => "balanced",
            Tier::Performance => "performance",
            Tier::Extreme => "extreme",
        }.to_string();

        if *current_mode_lock != active_mode {
            self.log_to_file(&format!("🚀 Transition: {} -> {}", *current_mode_lock, active_mode));
            *current_mode_lock = active_mode.clone();
            
            match tier {
                Tier::Eco => {
                    let _ = self.apply_governor_str("powersave");
                    let _ = self.apply_epp(EnergyPreference::Power);
                    let _ = self.apply_epb(15);
                },
                Tier::Balanced => {
                    let _ = self.apply_governor_str("powersave");
                    let _ = self.apply_epp(EnergyPreference::BalancePower);
                    let _ = self.apply_epb(8);
                },
                Tier::Performance => {
                    let _ = self.apply_governor_str("performance");
                    let _ = self.apply_epp(EnergyPreference::BalancePerformance);
                    let _ = self.apply_epb(4);
                },
                Tier::Extreme => {
                    let _ = self.apply_governor_str("performance");
                    let _ = self.apply_epp(EnergyPreference::Performance);
                    let _ = self.apply_epb(0);
                },
            }
        }

        let prev_unpark = self.current_unpark_count.load(Ordering::Relaxed);

        if accel.abs() > 55.0 {
            self.log_to_file("⏳ Skipping Core Parking due to reading instability spike.");
        } else if unpark_count < prev_unpark && current_load > 65.0 {
            self.log_to_file("⏳ Skipping Core Parking due to high instantaneous load.");
        } else {
            self.park_cores(Some(unpark_count));
            self.current_unpark_count.store(unpark_count, Ordering::Relaxed);
        }

        let is_gaming = false;

        if tier == Tier::Performance || tier == Tier::Extreme || is_gaming {
            std::time::Duration::from_secs(1)
        } else {
            std::time::Duration::from_secs(5)
        }
    }

    pub fn apply_governor_str(&self, gov: &str) -> Result<(), String> {
        self.write_to_all_cpus("cpufreq/scaling_governor", gov)
    }

    pub fn apply_governor(&self, gov: Governor) -> Result<(), String> {
        let gov_str = match gov {
            Governor::Performance => "performance",
            Governor::Powersave => "powersave",
            Governor::Schedutil => "schedutil",
        };
        
        self.apply_governor_str(gov_str)
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
                        // The error on locked/parked cores is safely ignored here
                        if let Err(e) = self.write_sysfs_smart(&full_path, value) {
                            errors.push(e);
                        }
                    }
                }
            }
        }
        
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }
}