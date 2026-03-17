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
    fn calculate_dynamic_state(
        &self, 
        rolling_avg: f32, 
        _accel: f32, // Ignored: Pure EMA prevents hotplug anomalies
        battery_level: f32, 
        is_charging: bool, 
        mode: &str
    ) -> (usize, bool) {
        if mode == "performance" {
            return (self.total_cores, true); 
        }

        let max_stage = (self.total_cores.saturating_sub(2)) * 2 + 1;
        let mut current_stage = self.current_stage.load(Ordering::Relaxed);

        // 1. EVALUATE THE EMA (Exponential Moving Average)
        let critical_upper = 90.0;
        let upper_bound = 70.0;
        let lower_bound = 25.0;

        if rolling_avg > critical_upper {
            // Massive sustained load -> Jump 2 stages (e.g., add Core + Turbo instantly)
            current_stage = current_stage.saturating_add(2).min(max_stage);
        } else if rolling_avg > upper_bound {
            // High sustained load -> Climb 1 stage
            current_stage = current_stage.saturating_add(1).min(max_stage);
        } else if rolling_avg < lower_bound {
            // Idle load -> Descend 1 stage gracefully
            current_stage = current_stage.saturating_sub(1);
        }
        // Between 25.0 and 70.0 is the Deadband. The stage remains perfectly locked.

        // 2. APPLY MODE CONSTRAINTS
        let mut active_max_stage = max_stage;
        if mode == "auto" && !is_charging && battery_level < 20.0 {
            // Cap at 75% hardware, Turbo OFF
            let max_cores_capped = (self.total_cores as f32 * 0.75).ceil() as usize;
            active_max_stage = (max_cores_capped.saturating_sub(2)) * 2; 
        }
        
        if mode == "efficiency" {
            // Force Turbo OFF for all core counts (Stages must be even numbers)
            if current_stage % 2 != 0 {
                current_stage = current_stage.saturating_sub(1);
            }
        }

        current_stage = current_stage.min(active_max_stage);
        
        // Save the step we are currently resting on
        self.current_stage.store(current_stage, Ordering::Relaxed);

        // 3. TRANSLATE STAGE TO HARDWARE INSTRUCTIONS
        // Stage 0 -> 2 Cores, Turbo OFF
        // Stage 1 -> 2 Cores, Turbo ON
        // Stage 2 -> 3 Cores, Turbo OFF
        // Stage 3 -> 3 Cores, Turbo ON ...
        let target_cores = 2 + (current_stage / 2);
        let turbo = (current_stage % 2) != 0;

        (target_cores, turbo)
    }

    pub fn handle_state_change(&self, metrics: &SystemMetrics) -> std::time::Duration {
        let config = AppConfig::load();
        
        let current_load = metrics.total_cpu_usage;
        let prev_bits = self.prev_cpu_usage.load(Ordering::Relaxed);
        let prev_load = f32::from_bits(prev_bits);
        let accel = current_load - prev_load;
        self.prev_cpu_usage.store(current_load.to_bits(), Ordering::Relaxed);

        let alpha = 0.15_f32; 
        let hist_bits = self.rolling_load_avg.load(Ordering::Relaxed);
        let prev_avg = f32::from_bits(hist_bits);
        let rolling_avg = (alpha * current_load) + ((1.0 - alpha) * prev_avg);
        self.rolling_load_avg.store(rolling_avg.to_bits(), Ordering::Relaxed);

        let battery_level = metrics.battery_level.unwrap_or(100.0);
        let is_charging = metrics.is_charging.unwrap_or(false);
        let cpu_temp = metrics.cpu_temperature.unwrap_or(0.0);

        let mut active_mode = config.operation_mode.clone();

        if cpu_temp > 95.0 {
            active_mode = "efficiency".to_string();
            self.log_to_file(&format!("⚠️ Thermal Throttle: {:.1}°C - Forcing Efficiency", cpu_temp));
        } else if !is_charging && battery_level <= 10.0 {
            active_mode = "efficiency".to_string();
            self.log_to_file(&format!("🛡️ Battery Critical: {:.1}% - Forcing Efficiency", battery_level));
        }

        let (target_cores, target_turbo) = self.calculate_dynamic_state(
            rolling_avg, 
            accel, 
            battery_level, 
            is_charging, 
            &active_mode
        );

        let _ = self.set_turbo(target_turbo);

        let current_unparked = self.current_unpark_count.load(Ordering::Relaxed);
        if target_cores != current_unparked {
            self.park_cores(Some(target_cores));
            self.current_unpark_count.store(target_cores, Ordering::Relaxed);
            
            // Log the Stage shift for easy debugging
            let current_stage = self.current_stage.load(Ordering::Relaxed);
            self.log_to_file(&format!("Staircase Step [{}]: {} Cores, Turbo {} (EMA: {:.1}%)", 
                current_stage, target_cores, if target_turbo { "ON" } else { "OFF" }, rolling_avg));
        }

        let mut current_mode_lock = self.current_mode.lock().unwrap();
        if *current_mode_lock != active_mode {
            self.log_to_file(&format!("🚀 Engine Intent Transition: {} -> {}", *current_mode_lock, active_mode));
            *current_mode_lock = active_mode.clone();

            match active_mode.as_str() {
                "performance" => {
                    let _ = self.apply_governor_str("performance");
                    let _ = self.apply_epp(EnergyPreference::Performance);
                    let _ = self.apply_epb(0);
                    let _ = self.write_sysfs_smart("/sys/devices/system/cpu/intel_pstate/max_perf_pct", "100");
                    
                    let _ = std::process::Command::new("sh").arg("-c").arg("iw dev $(ip route show default | awk '{print $5}') set power_save off 2>/dev/null").spawn();
                    let _ = std::process::Command::new("rfkill").arg("unblock").arg("bluetooth").spawn();
                },
                "efficiency" => {
                    let _ = self.apply_governor_str("powersave");
                    let _ = self.apply_epp(EnergyPreference::Power);
                    let _ = self.apply_epb(15);
                    
                    let _ = std::process::Command::new("sh").arg("-c").arg("iw dev $(ip route show default | awk '{print $5}') set power_save on 2>/dev/null").spawn();
                    let _ = std::process::Command::new("sh").arg("-c").arg("bluetoothctl info | grep -q 'Connected: yes' || rfkill block bluetooth 2>/dev/null").spawn();
                },
                _ => {} 
            }
        }

        if active_mode == "auto" {
            if rolling_avg > 60.0 {
                let _ = self.apply_governor_str("performance");
                let _ = self.apply_epp(EnergyPreference::BalancePerformance);
                let _ = self.apply_epb(4);
            } else {
                let _ = self.apply_governor_str("powersave");
                let _ = self.apply_epp(EnergyPreference::BalancePower);
                let _ = self.apply_epb(8);
            }
        }

        self.set_usb_autosuspend(config.usb_autosuspend);
        self.set_sata_alpm(config.sata_alpm);

        let b = battery::get_vendor_battery();
        let _ = b.set_thresholds(0, config.battery_threshold);

        if active_mode == "performance" || rolling_avg > 40.0 {
            std::time::Duration::from_secs(1) 
        } else {
            std::time::Duration::from_secs(3) 
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