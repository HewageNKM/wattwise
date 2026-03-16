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

use std::sync::atomic::{AtomicU32, Ordering};

use std::sync::Mutex;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tier {
    Eco,
    Balanced,
    Performance,
    Extreme,
}

pub struct PowerManager {
    prev_cpu_usage: AtomicU32,
    rolling_load_avg: AtomicU32,
    adaptive_eco_threshold: AtomicU32,
    adaptive_balance_threshold: AtomicU32,
    total_cores: usize,
    
    current_tier: Mutex<Option<Tier>>,
    current_usb_autosuspend: std::sync::atomic::AtomicBool,
    current_sata_alpm: std::sync::atomic::AtomicBool,
    current_unpark_count: std::sync::atomic::AtomicUsize,
    current_turbo_failed: std::sync::atomic::AtomicBool,
}

impl PowerManager {
    pub fn new() -> Self {
        let total_cores = {
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

        Self {
            prev_cpu_usage: AtomicU32::new(0_f32.to_bits()),
            rolling_load_avg: AtomicU32::new(0_f32.to_bits()),
            adaptive_eco_threshold: AtomicU32::new(10_f32.to_bits()),
            adaptive_balance_threshold: AtomicU32::new(40_f32.to_bits()),
            total_cores,
            current_tier: Mutex::new(None),
            current_usb_autosuspend: std::sync::atomic::AtomicBool::new(false),
            current_sata_alpm: std::sync::atomic::AtomicBool::new(false),
            current_unpark_count: std::sync::atomic::AtomicUsize::new(0),
            current_turbo_failed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn set_core_online(&self, id: usize, online: bool) -> Result<(), String> {
        if id == 0 { return Ok(()); } 
        let path = format!("/sys/devices/system/cpu/cpu{}/online", id);
        let val = if online { "1" } else { "0" };
        
        if std::path::Path::new(&path).exists() {
            match self.write_sysfs_smart(&path, val) {
                Ok(true) => {
                    self.log_to_file(&format!("✅ Core {} set online={}", id, online));
                    Ok(())
                },
                Ok(false) => Ok(()), // Already set
                Err(_) => {
                    // Fallback to tee if direct write fails
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
                                None => true, // unpark all
                            };
                            if let Err(e) = self.set_core_online(id, online) {
                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("/etc/zenith-energy/zenith-energy.log") 
                                {
                                    use std::io::Write;
                                    let _ = writeln!(file, "[{}] ❌ Core Parking Failed (Core {} online={}): {}", 
                                        chrono::Local::now().format("%H:%M:%S"), id, online, e);
                                }
                            }
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
        if self.current_usb_autosuspend.swap(enabled, Ordering::SeqCst) == enabled {
            return; // No change
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
            return; // No change
        }
        let val = if enabled { "med_power_with_dipm" } else { "max_performance" };
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("for d in /sys/class/scsi_host/host*/link_power_management_policy; do echo {} > \"$d\" 2>/dev/null; done", val))
            .status();
        self.log_to_file(&format!("💽 SATA ALPM: {}", if enabled { "Enabled (med_power)" } else { "Disabled (max_perf)" }));
    }

    fn log_to_file(&self, message: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/etc/zenith-energy/zenith-energy.log") 
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%H:%M:%S"), message);
        }
    }

    fn write_sysfs(&self, path: &str, value: &str) -> Result<(), String> {
        fs::write(path, value).map_err(|e| format!("Failed to write to {}: {}", path, e))
    }

    fn write_sysfs_smart(&self, path: &str, value: &str) -> Result<bool, String> {
        if let Ok(current) = fs::read_to_string(path) {
            if current.trim() == value {
                return Ok(false); // No change needed
            }
        }
        self.write_sysfs(path, value).map(|_| true)
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
        let paths = [
            "/sys/devices/system/cpu/intel_pstate/no_turbo",
            "/sys/devices/system/cpu/cpufreq/boost"
        ];
        
        for path in paths {
            if std::path::Path::new(path).exists() {
                if self.current_turbo_failed.load(Ordering::Relaxed) {
                    continue; // Skip repeatedly writing if previously failed (Permission Denied)
                }

                let val = if path.contains("boost") {
                    if enabled { "1" } else { "0" }
                } else {
                    if enabled { "0" } else { "1" }
                };
                
                match self.write_sysfs_smart(path, val) {
                    Ok(true) => self.log_to_file(&format!("✅ Turbo Boost set to {} ({})", enabled, path)),
                    Ok(false) => {}, // No change
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

    fn calculate_staircase_state(&self, load: f32) -> (usize, bool) {
        match load {
            l if l < 15.0 => (2, false),                   // Stage 1: 2 cores, Base
            l if l < 30.0 => (2, true),                    // Stage 2: 2 cores, Turbo
            l if l < 50.0 => (4.min(self.total_cores), false), // Stage 3: 4 cores, Base
            l if l < 70.0 => (4.min(self.total_cores), true),  // Stage 4: 4 cores, Turbo
            l if l < 85.0 => (self.total_cores, false),    // Stage 5: All cores, Base
            _ => (self.total_cores, true),                  // Stage 6: All cores, Turbo
        }
    }

    pub fn handle_state_change(&self, metrics: &SystemMetrics) -> std::time::Duration {
        let config = AppConfig::load();
        
        // 1. Calculate Acceleration Derivative & EWMA (Machine Learning)
        let current_load = metrics.total_cpu_usage;
        let prev_bits = self.prev_cpu_usage.load(Ordering::Relaxed);
        let prev_load = f32::from_bits(prev_bits);
        let accel = current_load - prev_load;
        self.prev_cpu_usage.store(current_load.to_bits(), Ordering::Relaxed);

        // EWMA calculation: rolling = alpha * current + (1 - alpha) * prev
        let alpha = 0.12_f32; // Lowered to 0.12 to increase resistance against 1-2 second spikes
        let hist_bits = self.rolling_load_avg.load(Ordering::Relaxed);
        let prev_avg = f32::from_bits(hist_bits);
        let rolling_avg = (alpha * current_load) + ((1.0 - alpha) * prev_avg);
        self.rolling_load_avg.store(rolling_avg.to_bits(), Ordering::Relaxed);

        // Lightweight Machine Learning (AIMD Adaptive Thresholds)
        let eco_bits = self.adaptive_eco_threshold.load(Ordering::Relaxed);
        let mut eco_thresh = f32::from_bits(eco_bits);

        let bal_bits = self.adaptive_balance_threshold.load(Ordering::Relaxed);
        let mut bal_thresh = f32::from_bits(bal_bits);

        if accel.abs() < 1.5 && current_load < 30.0 {
            // Stable node -> Incrementally raise thresholds (Additive Increase)
            eco_thresh = (eco_thresh + 0.1).min(18.0); // Cap 18%
            bal_thresh = (bal_thresh + 0.1).min(50.0); // Cap 50%
        } else if accel > 4.0 {
            // sudden workload spike -> Lower thresholds (Multiplicative Decrease)
            eco_thresh = (eco_thresh * 0.85).max(7.0);  // Min 7%
            bal_thresh = (bal_thresh * 0.90).max(30.0); // Min 30%
        }

        self.adaptive_eco_threshold.store(eco_thresh.to_bits(), Ordering::Relaxed);
        self.adaptive_balance_threshold.store(bal_thresh.to_bits(), Ordering::Relaxed);

        let battery_level = metrics.battery_level.unwrap_or(100.0);
        let is_charging = metrics.is_charging.unwrap_or(false);

        // 2. Determine Continuous Curve Tier (Fixed feedback loop via rolling_avg)
        let mut tier = if is_charging {
            Tier::Extreme // 🔌 AC Power: Unrestricted maximum performance
        } else {
            match rolling_avg {
                l if l < eco_thresh && accel < 3.0 => Tier::Eco,
                l if l < bal_thresh => Tier::Balanced,
                l if l < 75.0 => Tier::Performance,
                _ => Tier::Extreme,
            }
        };

        // 🌡️ 3. Thermal Safety Guard (Anti-Freeze override)
        let cpu_temp = metrics.cpu_temperature.unwrap_or(0.0);
        if cpu_temp > 85.0 {
            tier = Tier::Eco;
            self.log_to_file(&format!("⚠️ Thermal Throttle Engaged: {:.1}°C - Forcing Eco", cpu_temp));
            // Force disable turbo when thermal throttling is active
            if let Ok(mut current) = std::fs::OpenOptions::new().append(true).open("/etc/zenith-energy/zenith-energy.log") {
                use std::io::Write;
                let _ = writeln!(current, "[{}] ⚠️ Thermal Safety Throttle Overriding to Eco", chrono::Local::now().format("%H:%M:%S"));
            }
        }

        if let Ok(metadata) = std::fs::metadata("/etc/zenith-energy/zenith-energy.log") {
            if metadata.len() > 200_000 {
                let _ = std::fs::write("/etc/zenith-energy/zenith-energy.log", "");
            }
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/etc/zenith-energy/zenith-energy.log") 
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] Autopilot: Load={:.1}%, Accel={:.1}, Rolling={:.1}% | Tier={:?}", 
                chrono::Local::now().format("%H:%M:%S"), current_load, accel, rolling_avg, tier);
        }

        let mut is_gaming = false;
        if let Some(proc) = metrics.top_processes.first() {
            if self.get_process_category(&proc.name) == "gaming" {
                is_gaming = true;
            }
        }

        // (Variables moved to top)

        // Staircase Scaling State (Dampened by EWMA to prevent jitter)
        let (staircase_cores, staircase_turbo) = self.calculate_staircase_state(rolling_avg);
        let unpark_count = staircase_cores;

        // Always ensure battery threshold is set according to config
        let b = battery::get_vendor_battery();
        let _ = b.set_thresholds(0, config.battery_threshold);

        // 🟢 2. Profile Selection node
        let profile = if let Some(ov) = &config.manual_override {
            if ov == "performance" { config.ac_profile.clone() } else { config.bat_profile.clone() }
        } else if is_charging {
            config.ac_profile.clone()
        } else {
            config.bat_profile.clone()
        };

        // 🟠 3. Apply Base settings (Only if manual override is active)
        if config.manual_override.is_some() {
            let _ = self.apply_governor_str(&profile.governor);
        }
        let mut turbo = profile.turbo;
        
        if config.manual_override.is_some() {
            if profile.core_parking {
                self.park_cores(Some(2)); 
            } else {
                self.park_cores(None); 
            }
        }
        self.set_usb_autosuspend(profile.usb_autosuspend);
        self.set_sata_alpm(profile.sata_alpm);

        // 🔴 4. Hardware-Accelerated continuous Autopilot Overlays
        if config.manual_override.is_none() {
            let mut current_tier_lock = self.current_tier.lock().unwrap();
            let tier_changed = current_tier_lock.map_or(true, |t| t != tier);
            
            let prev_unpark = self.current_unpark_count.load(Ordering::Relaxed);
            let unpark_changed = prev_unpark != unpark_count;

            // Safety Caps: Battery < 10% forces Eco regardless of tier
            if !is_charging && battery_level <= 10.0 {
                if tier_changed {
                    self.log_to_file(&format!("🛡️ SAFETY GUARD ACTIVE: Battery {:.1}% forcing Eco Tier", battery_level));
                    let _ = self.apply_governor_str("powersave");
                    let _ = self.apply_epp(EnergyPreference::Power);
                    self.park_cores(Some(2));
                    self.current_unpark_count.store(2, Ordering::Relaxed);
                    *current_tier_lock = Some(Tier::Eco);
                }
                turbo = false;
            } else {
                if tier_changed {
                    self.log_to_file(&format!("🚀 Autopilot Transition: {:?} -> {:?}", *current_tier_lock, tier));
                    
                    // 🔔 Desktop Notification
                    let msg = match tier {
                        Tier::Eco => "🔋 Eco Mode Engaged - Battery Saver",
                        Tier::Balanced => "🟡 Balanced Mode Active",
                        Tier::Performance => "🔴 Performance Mode Active",
                        Tier::Extreme => "⚡ Extreme Mode Active - Full Boost",
                    };
                    let _ = std::process::Command::new("notify-send")
                        .arg("Zenith Energy")
                        .arg(msg)
                        .spawn();

                    match tier {
                        Tier::Eco => {
                            let _ = self.apply_governor_str("powersave");
                            let _ = self.apply_epp(EnergyPreference::Power);
                            let _ = self.apply_epb(15);
                            
                            // 📡 Advanced Peripheral Management
                            let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg("iw dev $(ip route show default | awk '{print $5}') set power_save on 2>/dev/null")
                                .spawn();
                            let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg("bluetoothctl info | grep -q 'Connected: yes' || rfkill block bluetooth 2>/dev/null")
                                .spawn();
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
                            let _ = self.write_sysfs_smart("/sys/devices/system/cpu/intel_pstate/max_perf_pct", "100");
                            self.park_cores(None);
                            self.current_unpark_count.store(self.total_cores, Ordering::Relaxed);

                            // 📡 Advanced Peripheral Management
                            let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg("iw dev $(ip route show default | awk '{print $5}') set power_save off 2>/dev/null")
                                .spawn();
                            let _ = std::process::Command::new("rfkill")
                                .arg("unblock")
                                .arg("bluetooth")
                                .spawn();
                        },
                    }
                    *current_tier_lock = Some(tier);
                }
                
                // Situational Core Allocation (Allocate resources for max perf/eff regardless of tier change)
                if tier != Tier::Extreme {
                    if unpark_changed {
                        if is_charging || battery_level > 30.0 {
                            self.park_cores(Some(unpark_count));
                        } else {
                            self.park_cores(None);
                        }
                        self.current_unpark_count.store(unpark_count, Ordering::Relaxed);
                    }
                }
                
                // Turbo state is driven by the staircase logic on demand
                turbo = staircase_turbo;
                
                // Safety restriction: Eco tier NEVER gets turbo regardless of staircase
                if tier == Tier::Eco {
                    turbo = false;
                }
            }
        }

        let _ = self.set_turbo(turbo);

        // 🔄 5. Return Adaptive Tick Cycle rate
        if tier == Tier::Performance || tier == Tier::Extreme || is_gaming {
            std::time::Duration::from_secs(1) // High intelligence reacts instantly Node
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
