use sysinfo::{System, ProcessesToUpdate};
use serde::{Serialize, Deserialize};
use crate::battery;
use crate::config::AppConfig;
use procfs::CurrentSI; 

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CpuCoreInfo {
    pub id: usize,
    pub usage: f32,
    pub frequency: u64,
    pub max_frequency: u64,
    pub temperature: Option<f32>,
    pub online: bool,
    pub governor: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_usage: f32,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMetrics {
    pub total_cpu_usage: f32,
    pub cores: Vec<CpuCoreInfo>,
    pub load_avg: (f64, f64, f64),
    pub uptime: u64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_usage: f32,
    pub battery_level: Option<f32>,
    pub is_charging: Option<bool>,
    pub battery_health: Option<f32>,
    pub battery_cycles: Option<u32>,
    pub battery_time_remaining: Option<f32>,
    pub battery_vendor: String,
    pub battery_voltage: Option<f32>,
    pub battery_current: Option<f32>,
    pub battery_capacity_design: Option<f32>,
    pub battery_capacity_full: Option<f32>,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
    pub model_name: Option<String>,
    pub technology: Option<String>,
    pub cpu_temperature: Option<f32>,
    pub battery_discharge_rate: Option<f32>,
    pub top_processes: Vec<ProcessInfo>,
    pub config: AppConfig,
    pub daemon_unpark_count: Option<u32>,
    pub daemon_max_perf_pct: Option<u32>,
    pub daemon_tier: Option<String>,
}

pub struct Monitor {
    sys: System,
    cached_disk: f32,
    last_disk_check: std::time::Instant,
    prev_total_ticks: u64,
    prev_active_ticks: u64,
    prev_core_ticks: Vec<(u64, u64)>,
    last_power_state: Option<bool>, 
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        Self { 
            sys,
            cached_disk: 0.0,
            last_disk_check: std::time::Instant::now() - std::time::Duration::from_secs(12),
            prev_total_ticks: 0,
            prev_active_ticks: 0,
            prev_core_ticks: Vec::new(),
            last_power_state: None,
        }
    }

    pub fn get_metrics(&mut self) -> SystemMetrics {
        self.sys.refresh_memory();
        let core_temp = self.get_cpu_temp();
        let mut total_cpu_usage = 0.0;
        let mut cores = Vec::new();

        if let Ok(stat) = procfs::KernelStats::current() {
            // Total CPU Usage
            if let Some(cpu) = stat.cpu_time.first() {
                let current_total = cpu.user + cpu.nice + cpu.system + cpu.idle + 
                                   cpu.iowait.unwrap_or(0) + cpu.irq.unwrap_or(0) + 
                                   cpu.softirq.unwrap_or(0) + cpu.steal.unwrap_or(0);
                let current_idle = cpu.idle + cpu.iowait.unwrap_or(0);
                let current_active = current_total - current_idle;

                let delta_total = current_total.saturating_sub(self.prev_total_ticks);
                let delta_active = current_active.saturating_sub(self.prev_active_ticks);
                if delta_total > 0 {
                    total_cpu_usage = (delta_active as f32 / delta_total as f32) * 100.0;
                }
                self.prev_total_ticks = current_total;
                self.prev_active_ticks = current_active;
            }

            // Per-Core Usage
            let mut core_usages = Vec::new();
            for (idx, cpu) in stat.cpu_time.iter().skip(1).enumerate() {
                let current_total = cpu.user + cpu.nice + cpu.system + cpu.idle + 
                                   cpu.iowait.unwrap_or(0) + cpu.irq.unwrap_or(0) + 
                                   cpu.softirq.unwrap_or(0) + cpu.steal.unwrap_or(0);
                let current_idle = cpu.idle + cpu.iowait.unwrap_or(0);
                let current_active = current_total - current_idle;

                let mut usage = 0.0;
                if self.prev_core_ticks.len() > idx {
                    let (prev_total, prev_active) = self.prev_core_ticks[idx];
                    let delta_total = current_total.saturating_sub(prev_total);
                    let delta_active = current_active.saturating_sub(prev_active);
                    if delta_total > 0 {
                        usage = (delta_active as f32 / delta_total as f32) * 100.0;
                    }
                    self.prev_core_ticks[idx] = (current_total, current_active);
                } else {
                    self.prev_core_ticks.push((current_total, current_active));
                }
                core_usages.push(usage);
            }

            if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
                let mut items: Vec<_> = entries.flatten().collect();
                items.sort_by_key(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                        name[3..].parse::<usize>().unwrap_or(0)
                    } else { 999 }
                });

                for entry in items {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(id) = name[3..].parse::<usize>() {
                            let freq_path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq", id);
                            let freq = std::fs::read_to_string(&freq_path)
                                .ok()
                                .and_then(|s| s.trim().parse::<u64>().ok())
                                .map(|f| f / 1000)
                                .unwrap_or(0); 
                            
                            let max_freq = std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq", id))
                                .ok()
                                .and_then(|s| s.trim().parse::<u64>().ok())
                                .map(|f| f / 1000)
                                .unwrap_or(5000);

                            let governor = std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor", id))
                                .unwrap_or_else(|_| "unknown".to_string())
                                .trim().to_string();

                            let online = std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/online", id))
                                .ok()
                                .and_then(|s| s.trim().parse::<u8>().ok())
                                .map(|o| o == 1)
                                .unwrap_or(true); // cpu0 might not have online file but is always online

                            cores.push(CpuCoreInfo {
                                id,
                                usage: core_usages.get(id).cloned().unwrap_or(0.0),
                                frequency: freq,
                                max_frequency: max_freq,
                                temperature: core_temp,
                                online,
                                governor,
                            });
                        }
                    }
                }
            }
        }

        if self.last_disk_check.elapsed() > std::time::Duration::from_secs(10) {
            let output = std::process::Command::new("df").arg("/").arg("--output=pcent").output();
            if let Ok(o) = output {
                self.cached_disk = String::from_utf8_lossy(&o.stdout).lines().nth(1)
                    .map(|s| s.trim().trim_end_matches('%').parse::<f32>().unwrap_or(0.0))
                    .unwrap_or(0.0);
            }
            self.last_disk_check = std::time::Instant::now();
        }

        self.sys.refresh_processes(ProcessesToUpdate::All);
        let mut procs: Vec<_> = self.sys.processes().iter().map(|(pid, proc)| ProcessInfo {
            name: proc.name().to_string_lossy().to_string(),
            cpu_usage: proc.cpu_usage(), 
            pid: pid.as_u32()
        }).collect();

        // Sort by CPU usage descending
        procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
        let top_processes = procs.into_iter().take(5).collect();

        let battery = battery::get_vendor_battery();
        let stats = battery.get_stats().ok();
        let load = System::load_average();

        SystemMetrics {
            total_cpu_usage,
            cores,
            load_avg: (load.one, load.five, load.fifteen),
            uptime: System::uptime(),
            memory_used: self.sys.used_memory(),
            memory_total: self.sys.total_memory(),
            disk_usage: self.cached_disk,
            battery_level: stats.as_ref().map(|s| s.level),
            is_charging: stats.as_ref().map(|s| s.is_charging),
            battery_health: stats.as_ref().and_then(|s| s.health),
            battery_cycles: stats.as_ref().and_then(|s| s.cycle_count),
            battery_time_remaining: stats.as_ref().and_then(|s| s.time_remaining),
            battery_vendor: stats.as_ref().map(|s| s.vendor.clone()).unwrap_or_else(|| "None".to_string()),
            battery_voltage: stats.as_ref().and_then(|s| s.voltage_now),
            battery_current: stats.as_ref().and_then(|s| s.current_now),
            battery_capacity_design: stats.as_ref().and_then(|s| s.capacity_design),
            battery_capacity_full: stats.as_ref().and_then(|s| s.capacity_full),
            manufacturer: stats.as_ref().and_then(|s| s.manufacturer.clone()),
            serial_number: stats.as_ref().and_then(|s| s.serial_number.clone()),
            model_name: stats.as_ref().and_then(|s| s.model_name.clone()),
            technology: stats.as_ref().and_then(|s| s.technology.clone()),
            cpu_temperature: core_temp,
            battery_discharge_rate: stats.as_ref().and_then(|s| {
                if let (Some(v), Some(c)) = (s.voltage_now, s.current_now) { Some(v * c) } else { None }
            }),
            top_processes,
            config: AppConfig::load(),
            daemon_unpark_count: self.read_state("unpark_count"),
            daemon_max_perf_pct: self.read_state("max_perf_pct"),
            daemon_tier: self.read_state_str("tier"),
        }
    }

    fn get_cpu_temp(&self) -> Option<f32> {
        if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(t) = std::fs::read_to_string(path.join("type")) {
                    let t_trim = t.trim();
                    if t_trim == "x86_pkg_temp" || t_trim == "TCPU" || t_trim == "acpitz" {
                        if let Ok(temp_str) = std::fs::read_to_string(path.join("temp")) {
                            if let Ok(temp_val) = temp_str.trim().parse::<f32>() {
                                return Some(temp_val / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn read_state(&self, key: &str) -> Option<u32> {
        std::fs::read_to_string("/run/wattwise.state").ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get(key).and_then(|k| k.as_u64()).map(|u| u as u32))
    }

    fn read_state_str(&self, key: &str) -> Option<String> {
        std::fs::read_to_string("/run/wattwise.state").ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get(key).and_then(|k| k.as_str()).map(|s| s.to_string()))
    }
}