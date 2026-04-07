use sysinfo::{System, ProcessesToUpdate};
use serde::{Serialize, Deserialize};
use crate::config::AppConfig;
use procfs::CurrentSI; 
use std::time::{SystemTime, UNIX_EPOCH};
use chrono;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub description: String,
}

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
    pub cpu_temperature: Option<f32>,
    pub top_processes: Vec<ProcessInfo>,
    pub config: AppConfig,
    pub daemon_unpark_count: Option<u32>,
    pub daemon_max_perf_pct: Option<u32>,
    pub daemon_tier: Option<String>,
    pub events: Vec<SystemEvent>,
}

pub struct Monitor {
    sys: System,
    cached_disk: f32,
    last_disk_check: std::time::Instant,
    prev_total_ticks: u64,
    prev_active_ticks: u64,
    prev_core_ticks: Vec<(u64, u64)>,
    last_mode: Option<String>,
    last_temp: Option<f32>,
    last_high_cpu_log: std::time::Instant,
    events: Vec<SystemEvent>,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        let mut monitor = Self { 
            sys,
            cached_disk: 0.0,
            last_disk_check: std::time::Instant::now() - std::time::Duration::from_secs(12),
            prev_total_ticks: 0,
            prev_active_ticks: 0,
            prev_core_ticks: Vec::new(),
            last_mode: None,
            last_temp: None,
            last_high_cpu_log: std::time::Instant::now() - std::time::Duration::from_secs(60),
            events: Vec::new(),
        };
        monitor.load_events_from_log();
        monitor
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

        let load = System::load_average();

        let config = AppConfig::load();

        SystemMetrics {
            total_cpu_usage,
            cores,
            load_avg: (load.one, load.five, load.fifteen),
            uptime: System::uptime(),
            memory_used: self.sys.used_memory(),
            memory_total: self.sys.total_memory(),
            disk_usage: self.cached_disk,
            cpu_temperature: core_temp,
            top_processes,
            events: self.update_events(config.operation_mode.clone(), core_temp, &top_processes),
            config,
            daemon_unpark_count: self.read_state("unpark_count"),
            daemon_max_perf_pct: self.read_state("max_perf_pct"),
            daemon_tier: self.read_state_str("tier"),
        }
    }

    fn update_events(&mut self, mode: String, current_temp: Option<f32>, top_procs: &Vec<ProcessInfo>) -> Vec<SystemEvent> {
        let now_unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut new_events = Vec::new();

        // 1. Mode Change Detection
        if let Some(prev_mode) = &self.last_mode {
            if mode != *prev_mode {
                new_events.push(SystemEvent {
                    timestamp: now_unix,
                    event_type: "MODE_SHIFT".to_string(),
                    description: format!("System strategy shifted to {}", mode.to_uppercase()),
                });
            }
        }
        self.last_mode = Some(mode);

        // 2. Thermal Spike Detection (> 5°C jump or > 75°C absolute)
        if let Some(temp) = current_temp {
            if let Some(prev_temp) = self.last_temp {
                if temp > prev_temp + 5.0 || (temp > 75.0 && prev_temp <= 75.0) {
                    new_events.push(SystemEvent {
                        timestamp: now_unix,
                        event_type: "THERMAL_SPIKE".to_string(),
                        description: format!("Thermal anomaly detected: {:.1}°C", temp),
                    });
                }
            }
            self.last_temp = Some(temp);
        }

        // 3. High Resource Process Detection
        if self.last_high_cpu_log.elapsed() > std::time::Duration::from_secs(30) {
            if let Some(p) = top_procs.first() {
                if p.cpu_usage > 90.0 {
                    new_events.push(SystemEvent {
                        timestamp: now_unix,
                        event_type: "RESOURCE_HEAVY".to_string(),
                        description: format!("Process '{}' (PID {}) is consuming > 90% CPU", p.name, p.pid),
                    });
                    self.last_high_cpu_log = std::time::Instant::now();
                }
            }
        }

        for ev in &new_events {
            self.events.push(ev.clone());
            self.log_to_file(ev);
        }

        // Limit event history to last 50 events in memory
        if self.events.len() > 50 {
            self.events.remove(0);
        }

        self.events.clone()
    }

    fn log_to_file(&self, event: &SystemEvent) {
        use std::io::Write;
        let log_path = "/var/log/wattwise.log";
        
        // Check size and rotate if > 1MB
        if let Ok(meta) = std::fs::metadata(log_path) {
            if meta.len() > 1024 * 1024 {
                let _ = std::fs::rename(log_path, format!("{}.old", log_path));
            }
        }

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path) 
        {
            let now = SystemTime::now();
            let datetime: chrono::DateTime<chrono::Local> = now.into();
            let timestamp = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
            
            let line = format!("[{}] {}: {}\n", 
                timestamp,
                event.event_type, 
                event.description
            );
            let _ = file.write_all(line.as_bytes());
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

    fn load_events_from_log(&mut self) {
        let log_path = "/var/log/wattwise.log";
        if let Ok(content) = std::fs::read_to_string(log_path) {
            let mut loaded_events = Vec::new();
            for line in content.lines().rev().take(50) {
                // Parse line: "[2026-03-20 00:50:00] TYPE: DESCRIPTION"
                if line.starts_with('[') && line.contains(']') {
                    let parts: Vec<&str> = line.splitn(2, ']').collect();
                    if parts.len() == 2 {
                        let ts_str = parts[0].trim_start_matches('[');
                        let rest = parts[1].trim();
                        let type_desc: Vec<&str> = rest.splitn(2, ':').collect();
                        if type_desc.len() == 2 {
                            let event_type = type_desc[0].trim().to_string();
                            let description = type_desc[1].trim().to_string();
                            
                            let timestamp = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S")
                                .ok()
                                .map(|dt| dt.and_utc().timestamp() as u64)
                                .unwrap_or(0);
                                
                            loaded_events.push(SystemEvent {
                                timestamp,
                                event_type,
                                description,
                            });
                        }
                    }
                }
            }
            // Add loaded events to self.events if they are not already there
            for ev in loaded_events {
                if !self.events.iter().any(|e| e.timestamp == ev.timestamp && e.description == ev.description) {
                    self.events.push(ev);
                }
            }
            self.events.sort_by_key(|e| e.timestamp);
        }
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