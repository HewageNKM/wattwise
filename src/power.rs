use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LOW_LOAD_THRESHOLD_TICKS: usize = 12; 
const THERMAL_CUTOFF_CELSIUS: f32 = 72.0; 
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
    high_load_ticks: AtomicUsize,
    current_unpark_count: AtomicUsize,
    prev_usb_state: std::sync::atomic::AtomicBool,
    prev_sata_state: std::sync::atomic::AtomicBool,
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
            high_load_ticks: AtomicUsize::new(0),
            current_unpark_count: AtomicUsize::new(cores),
            prev_usb_state: std::sync::atomic::AtomicBool::new(false),
            prev_sata_state: std::sync::atomic::AtomicBool::new(false),
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
        
        let alpha = 0.25; 
        let prev_avg = f32::from_bits(self.rolling_load_avg.load(Ordering::Relaxed));
        let rolling_avg = (alpha * cpu_usage) + ((1.0 - alpha) * prev_avg);
        self.rolling_load_avg.store(rolling_avg.to_bits(), Ordering::Relaxed);

        if rolling_avg > 40.0 {
            let mut lhl = self.last_high_load.lock().unwrap();
            *lhl = Instant::now();
        }

        let mut target_tier = match rolling_avg {
            l if l < 15.0 => Tier::Eco,
            l if l < 45.0 => Tier::Balanced,
            l if l < 75.0 => Tier::Performance,
            _ => Tier::Extreme,
        };

        let mut max_cores_limit = self.total_cores;
        let mut force_all_cores = false;
        let mut force_turbo_off = false;

        match metrics.config.operation_mode.as_str() {
            "performance" => {
                target_tier = Tier::Extreme;
                force_all_cores = true;
            },
            "efficiency" => {
                target_tier = Tier::Eco;
                max_cores_limit = (self.total_cores / 2).max(CORE_MINIMUM);
                force_turbo_off = true;
            },
            _ => {}
        }

        let mut current_tier_lock = self.current_tier.lock().unwrap();
        let mut last_trans_lock = self.last_transition.lock().unwrap();

        println!("Autopilot: Load={:.1}%, Rolling={:.1}% | Tier={:?}", cpu_usage, rolling_avg, target_tier);

        if *current_tier_lock != target_tier && last_trans_lock.elapsed() > Duration::from_secs(2) {
            println!("🚀 Transition: {:?} -> {:?}", *current_tier_lock, target_tier);
            self.apply_tier_hardware(target_tier);
            
            if force_turbo_off {
                let _ = self.safe_write("/sys/devices/system/cpu/intel_pstate/no_turbo", "1");
                let _ = self.safe_write("/sys/devices/system/cpu/cpufreq/boost", "0");
            }

            *current_tier_lock = target_tier;
            *last_trans_lock = Instant::now();
        }

        let ideal_cores = if force_all_cores {
            self.total_cores
        } else {
            ((self.total_cores as f32 * (rolling_avg / 100.0)) * 1.25).ceil() as usize
        };
        let ideal_clamped = ideal_cores.clamp(CORE_MINIMUM, max_cores_limit);

        let current_unparked = self.current_unpark_count.load(Ordering::SeqCst);
        let mut final_core_target = current_unparked;
        
        if ideal_clamped > current_unparked {
            let ticks = self.high_load_ticks.fetch_add(1, Ordering::Relaxed);
            if ticks >= 1 { // Wait for 1 sustained tick (1s dampening)
                final_core_target = ideal_clamped;
                self.high_load_ticks.store(0, Ordering::Relaxed);
            }
            self.low_load_ticks.store(0, Ordering::Relaxed);
        } else if ideal_clamped < current_unparked {
            self.high_load_ticks.store(0, Ordering::Relaxed);
            let ticks = self.low_load_ticks.fetch_add(1, Ordering::Relaxed);
            if ticks >= LOW_LOAD_THRESHOLD_TICKS {
                final_core_target = ideal_clamped;
                self.low_load_ticks.store(0, Ordering::Relaxed);
            }
        }

        if final_core_target != current_unparked {
            self.park_cores_safe(final_core_target);
        }

        let state_json = format!("{{\"unpark_count\": {}, \"tier\": \"{:?}\"}}", final_core_target, target_tier);
        let _ = std::fs::write("/run/zenith-energy.state", state_json);

        if metrics.config.usb_autosuspend != self.prev_usb_state.load(Ordering::Relaxed) {
             self.set_usb_autosuspend(metrics.config.usb_autosuspend);
             self.prev_usb_state.store(metrics.config.usb_autosuspend, Ordering::Relaxed);
        }
        if metrics.config.sata_alpm != self.prev_sata_state.load(Ordering::Relaxed) {
             self.set_sata_alpm(metrics.config.sata_alpm);
             self.prev_sata_state.store(metrics.config.sata_alpm, Ordering::Relaxed);
        }

        if target_tier == Tier::Extreme { Duration::from_millis(500) } else { Duration::from_secs(1) }
    }

    pub fn set_usb_autosuspend(&self, enabled: bool) {
        let value = if enabled { "auto" } else { "on" };
        if let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") {
            for entry in entries.flatten() {
                let path = entry.path().join("power/control");
                let _ = self.safe_write(&path.to_string_lossy(), value);
            }
        }
    }

    pub fn set_sata_alpm(&self, enabled: bool) {
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

        for id in 1..self.total_cores {
            let online = id < target;
            let path = format!("/sys/devices/system/cpu/cpu{}/online", id);
            if let Ok(current) = std::fs::read_to_string(&path) {
                if current.trim() != if online { "1" } else { "0" } {
                    println!("✅ Core {} set online={}", id, online);
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
}