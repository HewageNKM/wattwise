use auto_cpufreq_rust::monitor::{self, Monitor};
use auto_cpufreq_rust::power::{PowerManager, Governor};
use std::sync::Mutex;
use tauri::State;

struct AppState {
    monitor: Mutex<Monitor>,
    power_manager: PowerManager,
}

#[tauri::command]
fn get_metrics(state: State<AppState>) -> Result<monitor::SystemMetrics, String> {
    let mut monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    Ok(monitor.get_metrics())
}

#[tauri::command]
fn set_governor(state: State<AppState>, governor: String) -> Result<(), String> {
    let gov = match governor.as_str() {
        "performance" => Governor::Performance,
        "powersave" => Governor::Powersave,
        "schedutil" => Governor::Schedutil,
        _ => return Err("Invalid governor".to_string()),
    };
    state.power_manager.apply_governor(gov)
}

#[tauri::command]
fn set_turbo(state: State<AppState>, enabled: bool) -> Result<(), String> {
    state.power_manager.set_turbo(enabled)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            monitor: Mutex::new(Monitor::new()),
            power_manager: PowerManager::new(),
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics,
            set_governor,
            set_turbo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
