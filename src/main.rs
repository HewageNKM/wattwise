use wattwise::monitor::{self, Monitor};
use wattwise::power::PowerManager;
use wattwise::config::AppConfig;
use std::sync::Mutex;
use std::process::Command;
use tauri::{Manager, State, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent}};

struct AppState {
    monitor: Mutex<Monitor>,
    power_manager: PowerManager,
}

#[tauri::command]
fn get_metrics(state: State<AppState>) -> Result<monitor::SystemMetrics, String> {
    let mut monitor = match state.monitor.lock() {
        Ok(guard) => guard,
        Err(poison) => poison.into_inner(),
    };
    Ok(monitor.get_metrics())
}



#[tauri::command]
fn set_usb_autosuspend(state: State<AppState>, enabled: bool) -> Result<(), String> {
    let mut config = AppConfig::load();
    config.usb_autosuspend = enabled;
    config.save()?;
    state.power_manager.set_usb_autosuspend(enabled);
    Ok(())
}

#[tauri::command]
fn set_sata_alpm(state: State<AppState>, enabled: bool) -> Result<(), String> {
    let mut config = AppConfig::load();
    config.sata_alpm = enabled;
    config.save()?;
    state.power_manager.set_sata_alpm(enabled);
    Ok(())
}

#[tauri::command]
fn set_operation_mode(_state: State<AppState>, mode: String) -> Result<(), String> {
    println!("🔔 Tauri: set_operation_mode called with mode: {}", mode);
    let mut config = AppConfig::load();
    config.operation_mode = mode;
    config.save()?;
    Ok(())
}

#[tauri::command]
fn get_logs() -> Result<String, String> {
    let output = Command::new("tail")
        // UPDATED: Pointing to /var/log/
        .args(["-n", "100", "/var/log/wattwise.log"])
        .output()
        .map_err(|e| e.to_string())?;
    
    let content = String::from_utf8_lossy(&output.stdout).to_string();
    if content.trim().is_empty() {
        Ok("Operational records file is empty. Standby for daemon loop offsets...".to_string())
    } else {
        Ok(content)
    }
}

static HIGH_TEMP_NOTIFIED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_daemon = args.iter().any(|arg| arg == "--daemon");

    if is_daemon {
        let monitor_mutex = Mutex::new(Monitor::new());
        let power_manager = PowerManager::new();
        println!("WattWise daemon starting...");
        
        loop {
            let metrics = {
                let mut monitor = monitor_mutex.lock().unwrap();
                monitor.get_metrics()
            };
            let sleep_duration = power_manager.handle_state_change(&metrics);
            std::thread::sleep(sleep_duration);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            monitor: Mutex::new(Monitor::new()),
            power_manager: PowerManager::new(),
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics,
            get_logs,
            set_usb_autosuspend,
            set_sata_alpm,
            set_operation_mode
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Background optimization loop
            std::thread::spawn(move || {
                let mut interval = std::time::Duration::from_secs(5);
                loop {
                    let metrics = {
                        let state: State<AppState> = app_handle.state();
                        let mut monitor = state.monitor.lock().unwrap();
                        monitor.get_metrics()
                    };
                    
                    let state: State<AppState> = app_handle.state();
                    interval = state.power_manager.handle_state_change(&metrics);



                    if let Some(temp) = metrics.cpu_temperature {
                        if temp >= 85.0 {
                            if !HIGH_TEMP_NOTIFIED.swap(true, std::sync::atomic::Ordering::SeqCst) {
                                let _ = Command::new("sh")
                                    .arg("-c")
                                    .arg("for u in $(who | awk '{print $1}'); do sudo -u $u DISPLAY=:0 notify-send 'WattWise' 'High Thermal State: Adaptive scaling forcing efficiency.' 2>/dev/null; done")
                                    .status();
                            }
                        } else if temp < 75.0 {
                            HIGH_TEMP_NOTIFIED.store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                    
                    std::thread::sleep(interval);
                }
            });

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show app", true, None::<&str>)?;
            let perf_i = MenuItem::with_id(app, "perf", "Performance Mode", true, None::<&str>)?;
            let save_i = MenuItem::with_id(app, "save", "Efficiency Mode", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&perf_i, &save_i, &show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            let window = app.get_webview_window("main").unwrap();
                            window.show().unwrap();
                            window.unminimize().unwrap();
                            window.set_focus().unwrap();
                        }
                        "perf" => {
                            let mut config = AppConfig::load();
                            config.operation_mode = "performance".to_string();
                            let _ = config.save();
                        }
                        "save" => {
                            let mut config = AppConfig::load();
                            config.operation_mode = "efficiency".to_string();
                            let _ = config.save();
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}