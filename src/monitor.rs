use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};
use serde::{Serialize, Deserialize};
use crate::battery::{self};
use crate::config::AppConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CpuCoreInfo {
    pub id: usize,
    pub usage: f32,
    pub frequency: u64,
    pub temperature: Option<f32>,
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
    pub cpu_temperature: Option<f32>,
    pub battery_discharge_rate: Option<f32>,
    pub top_processes: Vec<ProcessInfo>,
    pub config: AppConfig,
}

fn get_cpu_temp() -> Option<f32> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(t) = std::fs::read_to_string(path.join("type")) {
                let t_trim = t.trim();
                // Prefer x86_pkg_temp or TCPU
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

pub struct Monitor {
    sys: System,
    cached_disk: f32,
    last_disk_check: std::time::Instant,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything())
        );
        sys.refresh_all();
        Self { 
            sys,
            cached_disk: 0.0,
            last_disk_check: std::time::Instant::now() - std::time::Duration::from_secs(12)
        }
    }

    pub fn get_metrics(&mut self) -> SystemMetrics {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();

        let core_temp = get_cpu_temp();

        let mut cores = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
            let mut items: Vec<_> = entries.flatten().collect();
            items.sort_by_key(|e| {
                e.file_name().to_string_lossy()[3..].parse::<usize>().unwrap_or(0)
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
                            .unwrap_or(0); // 0 means offline/parked

                        let usage = self.sys.cpus().get(id).map(|c| c.cpu_usage()).unwrap_or(0.0);

                        cores.push(CpuCoreInfo {
                            id,
                            usage,
                            frequency: freq,
                            temperature: core_temp,
                        });
                    }
                }
            }
        }

        let load = System::load_average();

        let mut bat_level = None;
        let mut charging = None;
        let mut health = None;
        let mut voltage = None;
        let mut current = None;
        let mut cap_design = None;
        let mut cap_full = None;
        let mut cycles = None;
        let mut time_rem = None;
        let mut vendor = "None".to_string();

        let battery = battery::get_vendor_battery();

        let mut discharge_rate = None;

        if let Ok(stats) = battery.get_stats() {
            bat_level = Some(stats.level);
            charging = Some(stats.is_charging);
            health = stats.health;
            cycles = stats.cycle_count;
            time_rem = stats.time_remaining;
            vendor = stats.vendor;
            voltage = stats.voltage_now;
            current = stats.current_now;
            cap_design = stats.capacity_design;
            cap_full = stats.capacity_full;

            if let (Some(v), Some(c)) = (stats.voltage_now, stats.current_now) {
                discharge_rate = Some(v * c);
            }
        }

        if self.last_disk_check.elapsed() > std::time::Duration::from_secs(10) {
            use std::process::Command;
            let disk_usage = Command::new("df")
                .arg("/")
                .arg("--output=pcent")
                .output()
                .ok()
                .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().nth(1)
                    .map(|s| s.trim().trim_end_matches('%').parse::<f32>().unwrap_or(0.0)))
                .unwrap_or(0.0);
            self.cached_disk = disk_usage;
            self.last_disk_check = std::time::Instant::now();
        }

        self.sys.refresh_processes();
        let mut procs: Vec<_> = self.sys.processes().iter().map(|(pid, proc)| ProcessInfo {
            name: proc.name().to_string(),
            cpu_usage: proc.cpu_usage(),
            pid: pid.as_u32()
        }).collect();
        procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
        let top_processes = procs.into_iter().take(4).collect::<Vec<_>>();

        SystemMetrics {
            total_cpu_usage: self.sys.global_cpu_info().cpu_usage(),
            cores,
            load_avg: (load.one, load.five, load.fifteen),
            uptime: System::uptime(),
            memory_used: self.sys.used_memory(),
            memory_total: self.sys.total_memory(),
            disk_usage: self.cached_disk,
            battery_level: bat_level,
            is_charging: charging,
            battery_health: health,
            battery_cycles: cycles,
            battery_time_remaining: time_rem,
            battery_vendor: vendor,
            battery_voltage: voltage,
            battery_current: current,
            battery_capacity_design: cap_design,
            battery_capacity_full: cap_full,
            cpu_temperature: core_temp,
            battery_discharge_rate: discharge_rate,
            top_processes,
            config: AppConfig::load(),
        }
    }
}
