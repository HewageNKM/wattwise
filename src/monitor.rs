use sysinfo::{System, RefreshKind, CpuRefreshKind};
use serde::{Serialize, Deserialize};
use crate::battery::{self, BatteryProvider};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CpuCoreInfo {
    pub id: usize,
    pub usage: f32,
    pub frequency: u64,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMetrics {
    pub total_cpu_usage: f32,
    pub cores: Vec<CpuCoreInfo>,
    pub load_avg: (f64, f64, f64),
    pub uptime: u64,
    pub battery_level: Option<f32>,
    pub is_charging: Option<bool>,
}

pub struct Monitor {
    sys: System,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
        );
        sys.refresh_all();
        Self { sys }
    }

    pub fn get_metrics(&mut self) -> SystemMetrics {
        self.sys.refresh_cpu();

        let cores = self.sys.cpus().iter().enumerate().map(|(id, cpu)| {
            CpuCoreInfo {
                id,
                usage: cpu.cpu_usage(),
                frequency: cpu.frequency(),
                temperature: None,
            }
        }).collect();

        let load = System::load_average();

        let mut bat_level = None;
        let mut charging = None;
        if let Ok(stats) = battery::GenericLinuxBattery::new().get_stats() {
            bat_level = Some(stats.level);
            charging = Some(stats.is_charging);
        }

        SystemMetrics {
            total_cpu_usage: self.sys.global_cpu_info().cpu_usage(),
            cores,
            load_avg: (load.one, load.five, load.fifteen),
            uptime: System::uptime(),
            battery_level: bat_level,
            is_charging: charging,
        }
    }
}
