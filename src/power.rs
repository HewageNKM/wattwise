use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LOW_LOAD_THRESHOLD_TICKS: usize = 12; 
const THERMAL_MIN_CELSIUS: f32 = 65.0; 
const THERMAL_MAX_CELSIUS: f32 = 85.0;
const CORE_MINIMUM: usize = 2;              
const TURBO_SUSTAIN_SEC: u64 = 15;           

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tier { Eco, Balanced, Performance, Extreme }

pub struct PowerManager {
    total_cores: usize,
    rolling_load_avg: AtomicU32,
    current_tier: Mutex<Tier>,
    last_transition: Mutex<Instant>,
    last_park_event: Mutex<Instant>,
    last_high_load: Mutex<Instant>, 
    low_load_ticks: AtomicUsize,
    current_unpark_count: AtomicUsize,
    prev_usb_state: std::sync::atomic::AtomicBool,
    prev_sata_state: std::sync::atomic::AtomicBool,
    prev_wifi_state: std::sync::atomic::AtomicBool,
    prev_bluetooth_state: std::sync::atomic::AtomicBool,
    burst_ticks: AtomicUsize,
    prev_load: AtomicU32,
}

impl PowerManager {
    pub fn new() -> Self {
        let cores = fs::read_dir("/sys/devices/system/cpu")
            .map(|entries| entries.flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit())
                })
                .count())
            .unwrap_or(4);

        let pm = Self {
            total_cores: cores,
            rolling_load_avg: AtomicU32::new(0),
            current_tier: Mutex::new(Tier::Balanced),
            last_transition: Mutex::new(Instant::now()),
            last_park_event: Mutex::new(Instant::now()),
            last_high_load: Mutex::new(Instant::now()),
            low_load_ticks: AtomicUsize::new(0),
            current_unpark_count: AtomicUsize::new(cores),
            prev_usb_state: std::sync::atomic::AtomicBool::new(false),
            prev_sata_state: std::sync::atomic::AtomicBool::new(false),
            prev_wifi_state: std::sync::atomic::AtomicBool::new(true),
            prev_bluetooth_state: std::sync::atomic::AtomicBool::new(true),
            burst_ticks: AtomicUsize::new(0),
            prev_load: AtomicU32::new(0),
        };
        pm.park_cores_safe(cores); // Force unpark on boot
        pm
    }

    fn safe_write(&self, path: &str, value: &str) -> Result<(), String> {
        let p = Path::new(path);
        if !p.exists() { return Ok(()); }
        if let Ok(current) = fs::read_to_string(path) {
            if current.trim() == value { return Ok(()); }
        }
        fs::write(path, value).map_err(|e| format!("{}: {}", path, e))
    }

    pub fn handle_state_change(&self, metrics: &crate::monitor::SystemMetrics) -> Duration {
        let cpu_usage = metrics.total_cpu_usage;
        let cpu_temp = metrics.cpu_temperature.unwrap_or(0.0);
        
        // --- 1. Load Averaging (EMA) ---
        let alpha = 0.25; 
        let prev_avg = f32::from_bits(self.rolling_load_avg.load(Ordering::Relaxed));
        let rolling_avg = (alpha * cpu_usage) + ((1.0 - alpha) * prev_avg);
        self.rolling_load_avg.store(rolling_avg.to_bits(), Ordering::Relaxed);

        if rolling_avg > 40.0 {
            let mut lhl = self.last_high_load.lock().unwrap();
            *lhl = Instant::now();
        }

        let mut max_cores_limit = self.total_cores;
        let mut force_all_cores = false;
        let mut force_turbo_off = false;
        
        let operation_mode = metrics.config.operation_mode.as_str();

        // --- 2. Profile & Strategy Selection ---
        let (target_tier, apply_eco_caps) = match operation_mode {
            "performance" => {
                force_all_cores = true;
                self.set_asus_fan_policy(1); // Turbo Boost
                (Tier::Extreme, false)
            },
            "efficiency" => {
                max_cores_limit = (self.total_cores / 2).max(CORE_MINIMUM);
                force_turbo_off = true;
                self.set_asus_fan_policy(2); // Silent Mode
                (Tier::Eco, true)
            },
            _ => { // Auto-Pilot
                self.set_asus_fan_policy(0); // Standard Mode
                let is_ac = metrics.is_on_ac;
                
                let tier = if is_ac {
                    match rolling_avg { // Performance-biased thresholds for AC
                        l if l < 10.0 => Tier::Eco,
                        l if l < 30.0 => Tier::Balanced,
                        l if l < 55.0 => Tier::Performance,
                        _ => Tier::Extreme,
                    }
                } else {
                    match rolling_avg { // Efficiency-biased thresholds for Battery
                        l if l < 20.0 => Tier::Eco,
                        l if l < 55.0 => Tier::Balanced,
                        l if l < 80.0 => Tier::Performance,
                        _ => Tier::Extreme,
                    }
                };
                (tier, !is_ac)
            }
        };

        // --- 3. Stability & Transitions ---
        let mut current_tier_lock = self.current_tier.lock().unwrap();
        let mut last_trans_lock = self.last_transition.lock().unwrap();

        if *current_tier_lock != target_tier && last_trans_lock.elapsed() > Duration::from_secs(3) {
            println!("🔄 MODE_SHIFT: {:?} -> {:?}", *current_tier_lock, target_tier);
            self.log_event("MODE_SHIFT", &format!("System strategy transitioned: {:?} -> {:?}", *current_tier_lock, target_tier));
            self.apply_tier_hardware(target_tier);
            
            if force_turbo_off {
                let _ = self.safe_write("/sys/devices/system/cpu/intel_pstate/no_turbo", "1");
                let _ = self.safe_write("/sys/devices/system/cpu/cpufreq/boost", "0");
            }

            *current_tier_lock = target_tier;
            *last_trans_lock = Instant::now();
        }

        // --- 4. Core Allocation (Optimized) ---
        let core_floor = if metrics.is_on_ac {
            // On AC, keep more cores active for better background/multitask throughput
            (self.total_cores as f32 * 0.75).ceil() as usize
        } else {
            match *current_tier_lock {
                Tier::Eco => CORE_MINIMUM,
                Tier::Balanced => (self.total_cores as f32 * 0.5).ceil() as usize,
                _ => self.total_cores,
            }
        };

        let current_load = metrics.total_cpu_usage;
        let prev_load = self.prev_load.swap(current_load as u32, Ordering::SeqCst);
        let load_jump = current_load - prev_load as f32;
        
        if load_jump > 20.0 {
            self.log_event("BURST_DETECTED", &format!("Significant load spike (+{:.1}%). Fast-tracking unpark.", load_jump));
            force_all_cores = true;
            self.burst_ticks.store(2, Ordering::SeqCst);
        } else {
            let bt = self.burst_ticks.load(Ordering::SeqCst);
            if bt > 0 {
                force_all_cores = true;
                self.burst_ticks.fetch_sub(1, Ordering::SeqCst);
            }
        }

        let ideal_cores = if force_all_cores {
            self.total_cores
        } else {
            // Non-linear scaling: More cores unparked earlier to prevent spike lag
            let scaling_factor = if rolling_avg > 30.0 { 1.5 } else { 1.2 };
            ((self.total_cores as f32 * (rolling_avg / 100.0)) * scaling_factor).ceil() as usize
        };
        
        let ideal_clamped = ideal_cores.clamp(core_floor, max_cores_limit);

        let current_unparked = self.current_unpark_count.load(Ordering::SeqCst);
        let mut final_core_target = current_unparked;
        
        if ideal_clamped > current_unparked {
            // Aggressive unparking for responsiveness
            final_core_target = ideal_clamped;
        } else if ideal_clamped < current_unparked {
            // Conservative parking for stability
            let ticks = self.low_load_ticks.fetch_add(1, Ordering::Relaxed);
            if ticks >= LOW_LOAD_THRESHOLD_TICKS {
                final_core_target = ideal_clamped;
                self.low_load_ticks.store(0, Ordering::Relaxed);
            }
        }

        if final_core_target != current_unparked {
            self.park_cores_safe(final_core_target);
        }

        // --- 5. Thermal Smoothing (PID-Lite) ---
        let mut throttling_level = 0.0;
        let mut target_max_perf = 100;
        let config = &metrics.config;
        
        if config.thermal_smoothing {
            if cpu_temp >= THERMAL_MIN_CELSIUS {
                // Linear ramp from 65°C (100%) to 85°C (50%)
                let range = THERMAL_MAX_CELSIUS - THERMAL_MIN_CELSIUS;
                let excess = (cpu_temp - THERMAL_MIN_CELSIUS).max(0.0);
                let throttle_factor = (excess / range).min(1.0);
                
                throttling_level = throttle_factor * 100.0;
                target_max_perf = (100.0 - (throttle_factor * 50.0)) as u32;
                
                if throttling_level > 5.0 {
                    self.log_event("THERMAL_SMOOTHING", &format!("Temp {:.1}°C: Smoothing performance to {}%.", cpu_temp, target_max_perf));
                }

                if cpu_temp >= THERMAL_MAX_CELSIUS {
                    self.log_event("THERMAL_LOCK", &format!("Critical Heat ({:.1}°C): Performance restricted to hardware floor.", cpu_temp));
                }
            }
        } else {
            // Legacy hard-cutoff logic if smoothing is disabled
            if cpu_temp >= 75.0 {
                target_max_perf = 60;
                throttling_level = 40.0;
            }
        }

        // Apply thermal limits
        let _ = self.safe_write("/sys/devices/system/cpu/intel_pstate/max_perf_pct", &target_max_perf.to_string());
        if target_max_perf < 100 {
            let _ = self.safe_write("/sys/devices/system/cpu/intel_pstate/no_turbo", "1");
            let _ = self.safe_write("/sys/devices/system/cpu/cpufreq/boost", "0");
        }

        let state_json = format!(
            "{{\"unpark_count\": {}, \"tier\": \"{:?}\", \"max_perf_pct\": {}, \"throttling_level\": {:.1}}}", 
            final_core_target, target_tier, target_max_perf, throttling_level
        );
        let _ = std::fs::write("/run/wattwise.state", state_json);

        // --- 5. Advanced Hardware Tuning ---

        if apply_eco_caps {
            // Apply Power-Saving Caps
            self.apply_brightness_cap(40.0);
            if config.pcie_aspm { self.set_pcie_aspm("powersave"); }
            if !config.nmi_watchdog { self.set_nmi_watchdog(false); }
            if config.vm_writeback { self.set_vm_writeback(3000); }
            if config.laptop_mode { self.set_laptop_mode(5); }
            if !config.smt_status { self.set_smt_status(false); }

            if config.usb_autosuspend && !self.prev_usb_state.load(Ordering::Relaxed) {
                self.set_usb_autosuspend(true);
                self.prev_usb_state.store(true, Ordering::Relaxed);
            }
            if config.sata_alpm && !self.prev_sata_state.load(Ordering::Relaxed) {
                self.set_sata_alpm(true);
                self.prev_sata_state.store(true, Ordering::Relaxed);
            }

            // Intelligent Radio Control (Only disable if NOT connected)
            if !config.wifi_enabled && self.prev_wifi_state.load(Ordering::Relaxed) {
                if !self.is_wifi_connected() {
                    self.set_wifi_state(false);
                    self.prev_wifi_state.store(false, Ordering::Relaxed);
                    self.log_event("IDLE_RADIO", "No WiFi connection active. Radio disabled for power saving.");
                }
            }
            if !config.bluetooth_enabled && self.prev_bluetooth_state.load(Ordering::Relaxed) {
                if !self.is_bluetooth_connected() {
                    self.set_bluetooth_state(false);
                    self.prev_bluetooth_state.store(false, Ordering::Relaxed);
                    self.log_event("IDLE_RADIO", "No Bluetooth devices connected. Radio disabled for power saving.");
                }
            }
        } else {
            // Restore Performance States (or if on AC)
            if metrics.is_on_ac || cpu_temp < THERMAL_MIN_CELSIUS - 5.0 {
                self.set_pcie_aspm(if metrics.is_on_ac { "performance" } else { "powersave" });
                self.set_nmi_watchdog(true);
                self.set_vm_writeback(1500); 
                self.set_laptop_mode(0); 
                self.set_smt_status(true);
                
                // On AC, unblock radios immediately
                if metrics.is_on_ac {
                    if !self.prev_wifi_state.load(Ordering::Relaxed) {
                        self.set_wifi_state(true);
                        self.prev_wifi_state.store(true, Ordering::Relaxed);
                    }
                    if !self.prev_bluetooth_state.load(Ordering::Relaxed) {
                        self.set_bluetooth_state(true);
                        self.prev_bluetooth_state.store(true, Ordering::Relaxed);
                    }
                }

                if self.prev_usb_state.load(Ordering::Relaxed) != (config.usb_autosuspend && apply_eco_caps) {
                    let target = config.usb_autosuspend && apply_eco_caps;
                    self.set_usb_autosuspend(target);
                    self.prev_usb_state.store(target, Ordering::Relaxed);
                }
                if self.prev_sata_state.load(Ordering::Relaxed) != (config.sata_alpm && apply_eco_caps) {
                    let target = config.sata_alpm && apply_eco_caps;
                    self.set_sata_alpm(target);
                    self.prev_sata_state.store(target, Ordering::Relaxed);
                }
            }
        }

        if target_tier == Tier::Extreme { Duration::from_millis(500) } else { Duration::from_secs(1) }
    }

    pub fn apply_brightness_cap(&self, max_percentage: f32) {
        if let Ok(entries) = std::fs::read_dir("/sys/class/backlight") {
            for entry in entries.flatten() {
                let max_path = entry.path().join("max_brightness");
                let cur_path = entry.path().join("brightness");
                if let (Ok(max_str), Ok(cur_str)) = (std::fs::read_to_string(&max_path), std::fs::read_to_string(&cur_path)) {
                    if let (Ok(max_val), Ok(cur_val)) = (max_str.trim().parse::<u32>(), cur_str.trim().parse::<u32>()) {
                        let cap = (max_val as f32 * (max_percentage / 100.0)) as u32;
                        if cur_val > cap {
                            let _ = self.safe_write(&cur_path.to_string_lossy(), &cap.to_string());
                            println!("🔆 Brightness capped to {}% ({} max={})", max_percentage, cap, max_val);
                        }
                    }
                }
            }
        }
    }

    pub fn set_pcie_aspm(&self, policy: &str) {
        self.log_event("HW_POLICY", &format!("PCIe ASPM policy set to {}", policy));
        let _ = self.safe_write("/sys/module/pcie_aspm/parameters/policy", policy);
    }

    pub fn set_nmi_watchdog(&self, enabled: bool) {
        self.log_event("HW_POLICY", &format!("Kernel NMI Watchdog -> {}", if enabled { "ENABLED" } else { "DISABLED" }));
        let _ = self.safe_write("/proc/sys/kernel/nmi_watchdog", if enabled { "1" } else { "0" });
    }

    pub fn set_vm_writeback(&self, centisecs: u32) {
        let _ = self.safe_write("/proc/sys/vm/dirty_writeback_centisecs", &centisecs.to_string());
    }

    pub fn set_laptop_mode(&self, mode: u32) {
        self.log_event("HW_POLICY", &format!("VM Laptop Mode tier set to {}", mode));
        let _ = self.safe_write("/proc/sys/vm/laptop_mode", &mode.to_string());
    }

    pub fn set_smt_status(&self, enabled: bool) {
        self.log_event("HW_POLICY", &format!("SMT (Hyper-Threading) -> {}", if enabled { "ON" } else { "OFF" }));
        let _ = self.safe_write("/sys/devices/system/cpu/smt/control", if enabled { "on" } else { "off" });
    }

    pub fn set_usb_autosuspend(&self, enabled: bool) {
        self.log_event("HW_POLICY", &format!("USB Bus Autosuspend -> {}", if enabled { "ACTIVE" } else { "INACTIVE" }));
        let value = if enabled { "auto" } else { "on" };
        if let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") {
            for entry in entries.flatten() {
                let path = entry.path().join("power/control");
                let _ = self.safe_write(&path.to_string_lossy(), value);
            }
        }
    }

    pub fn set_sata_alpm(&self, enabled: bool) {
        self.log_event("HW_POLICY", &format!("SATA Aggressive Link Power -> {}", if enabled { "ENABLED" } else { "DISABLED" }));
        let value = if enabled { "med_power_with_dipm" } else { "max_performance" };
        if let Ok(entries) = std::fs::read_dir("/sys/class/scsi_host") {
            for entry in entries.flatten() {
                let path = entry.path().join("link_power_management_policy");
                let _ = self.safe_write(&path.to_string_lossy(), value);
            }
        }
    }

    fn park_cores_safe(&self, target: usize) {
        let mut last_park = self.last_park_event.lock().unwrap();
        if last_park.elapsed() < Duration::from_secs(3) { return; }

        self.log_event("SCHEDULER", &format!("Targeting {} unparked cores for current load strategy.", target));

        for id in 1..self.total_cores {
            let online = id < target;
            let path = format!("/sys/devices/system/cpu/cpu{}/online", id);
            if let Ok(current) = std::fs::read_to_string(&path) {
                let was_online = current.trim() == "1";
                if was_online != online {
                    self.log_event("CORE_SHIFT", &format!("CPU Core {}: {}", id, if online { "ONLINE" } else { "OFFLINE (PARKED)" }));
                }
            }
            let _ = self.safe_write(&path, if online { "1" } else { "0" });
        }
        self.current_unpark_count.store(target, Ordering::SeqCst);
        *last_park = Instant::now();
    }

    fn apply_tier_hardware(&self, tier: Tier) {
        match tier {
            Tier::Eco => {
                let _ = self.write_to_all_possible("cpufreq/scaling_governor", "powersave");
                let _ = self.write_to_all_possible("cpufreq/energy_performance_preference", "power");
                let _ = self.set_turbo_dynamic(false);
            },
            Tier::Balanced => {
                let _ = self.write_to_all_possible("cpufreq/scaling_governor", "powersave");
                let _ = self.write_to_all_possible("cpufreq/energy_performance_preference", "balance_power");
                let _ = self.set_turbo_dynamic(false);
            },
            Tier::Performance => {
                let _ = self.write_to_all_possible("cpufreq/scaling_governor", "performance");
                let _ = self.write_to_all_possible("cpufreq/energy_performance_preference", "balance_performance");
                let _ = self.set_turbo_dynamic(true);
            },
            Tier::Extreme => {
                let _ = self.write_to_all_possible("cpufreq/scaling_governor", "performance");
                let _ = self.write_to_all_possible("cpufreq/energy_performance_preference", "performance");
                let _ = self.set_turbo_dynamic(true);
            }
        }
    }

    pub fn enforce_exclusivity(&self) {
        let services = ["tlp", "auto-cpufreq", "power-profiles-daemon", "laptop-mode-tools", "thermald"];
        self.log_event("SYS_INIT", "Enforcing system exclusivity: Scanning for competing power managers...");
        
        for svc in services {
            // Check if service exists
            let status = std::process::Command::new("systemctl")
                .args(["list-unit-files", &format!("{}.service", svc)])
                .output();
                
            if let Ok(output) = status {
                if String::from_utf8_lossy(&output.stdout).contains(&format!("{}.service", svc)) {
                    self.log_event("CONFLICT_RESOLVED", &format!("Stopping and masking competing service: {}", svc));
                    let _ = std::process::Command::new("systemctl").args(["stop", svc]).status();
                    let _ = std::process::Command::new("systemctl").args(["disable", svc]).status();
                    let _ = std::process::Command::new("systemctl").args(["mask", svc]).status();
                }
            }
        }
    }

    fn set_turbo_dynamic(&self, request_on: bool) -> Result<(), String> {
        let lhl = self.last_high_load.lock().unwrap();
        let sustained_on = lhl.elapsed() < Duration::from_secs(TURBO_SUSTAIN_SEC);
        let final_state = request_on || sustained_on;
        let _ = self.safe_write("/sys/devices/system/cpu/intel_pstate/no_turbo", if final_state { "0" } else { "1" });
        let _ = self.safe_write("/sys/devices/system/cpu/cpufreq/boost", if final_state { "1" } else { "0" });
        let _ = self.safe_write("/sys/devices/system/cpu/intel_pstate/max_perf_pct", if final_state { "100" } else { "70" });
        Ok(())
    }

    fn write_to_all_possible(&self, subpath: &str, value: &str) -> Result<(), String> {
        let cpu_base = "/sys/devices/system/cpu";
        if let Ok(entries) = fs::read_dir(cpu_base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                    let full_path = format!("{}/{}/{}", cpu_base, name, subpath);
                    let _ = self.safe_write(&full_path, value);
                }
            }
        }
        Ok(())
    }

    pub fn set_asus_fan_policy(&self, policy: u32) {
        let _ = self.safe_write("/sys/devices/platform/asus-nb-wmi/throttle_thermal_policy", &policy.to_string());
    }


    pub fn is_wifi_connected(&self) -> bool {
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('w') { // Typically wlan0, wlo1, etc.
                    let carrier_path = format!("/sys/class/net/{}/carrier", name);
                    if let Ok(content) = fs::read_to_string(carrier_path) {
                        if content.trim() == "1" { return true; }
                    }
                }
            }
        }
        false
    }

    pub fn is_bluetooth_connected(&self) -> bool {
        if let Ok(entries) = fs::read_dir("/sys/class/bluetooth") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("hci") {
                    let conn_path = format!("/sys/class/bluetooth/{}/conn_count", name);
                    if let Ok(content) = fs::read_to_string(conn_path) {
                        if let Ok(count) = content.trim().parse::<u32>() {
                            if count > 0 { return true; }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn set_wifi_state(&self, enabled: bool) {
        let _ = std::process::Command::new("rfkill")
            .args([if enabled { "unblock" } else { "block" }, "wifi"])
            .status();
    }

    pub fn set_bluetooth_state(&self, enabled: bool) {
        let _ = std::process::Command::new("rfkill")
            .args([if enabled { "unblock" } else { "block" }, "bluetooth"])
            .status();
    }

    pub fn apply_charge_threshold(&self, limit: u32) {
        // Multi-vendor detection for battery charge thresholds
        let threshold_paths = [
            "/sys/class/power_supply/BAT0/charge_control_end_threshold",      // Newer ASUS/Lenovo
            "/sys/class/power_supply/BAT1/charge_control_end_threshold",
            "/sys/class/power_supply/BAT0/charge_stop_threshold",             // ThinkPad
            "/sys/devices/platform/asus-nb-wmi/charge_control_end_threshold", // Specific ASUS
            "/sys/devices/platform/samsung/battery_life_extender",            // Samsung (often 0/1)
        ];

        let mut applied = false;
        for path in threshold_paths {
            if Path::new(path).exists() {
                let _ = self.safe_write(path, &limit.to_string());
                applied = true;
            }
        }

        if applied {
            self.log_event("BATTERY_HEALTH", &format!("Applied battery charge threshold: {}%.", limit));
        }
    }

    fn log_event(&self, event_type: &str, description: &str) {
        use std::io::Write;
        let log_path = "/var/log/wattwise.log";
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path) 
        {
            let line = format!("[{}] {}: {}\n", timestamp, event_type, description);
            let _ = file.write_all(line.as_bytes());
        }
    }
}
