use auto_cpufreq_rust::monitor::{self, Monitor};
use auto_cpufreq_rust::power::{PowerManager, Governor};
use std::sync::Mutex;
use tauri::{Manager, State, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent}};

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
    let args: Vec<String> = std::env::args().collect();
    let is_daemon = args.iter().any(|arg| arg == "--daemon");

    if is_daemon {
        let monitor_mutex = Mutex::new(Monitor::new());
        let power_manager = PowerManager::new();
        println!("Zenith-Energy daemon starting...");
        
        loop {
            let metrics = {
                let mut monitor = monitor_mutex.lock().unwrap();
                monitor.get_metrics()
            };
            power_manager.handle_state_change(&metrics);
            std::thread::sleep(std::time::Duration::from_secs(5));
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
            set_governor,
            set_turbo
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Background optimization loop
            std::thread::spawn(move || {
                let interval = std::time::Duration::from_secs(5);
                loop {
                    let metrics = {
                        let state: State<AppState> = app_handle.state();
                        let mut monitor = state.monitor.lock().unwrap();
                        monitor.get_metrics()
                    };
                    
                    let state: State<AppState> = app_handle.state();
                    state.power_manager.handle_state_change(&metrics);
                    
                    std::thread::sleep(interval);
                }
            });

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show app", true, None::<&str>)?;
            let perf_i = MenuItem::with_id(app, "perf", "Performance Mode", true, None::<&str>)?;
            let save_i = MenuItem::with_id(app, "save", "Powersave Mode", true, None::<&str>)?;
            
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
                            let state: State<AppState> = app.state();
                            let _ = state.power_manager.apply_governor(Governor::Performance);
                        }
                        "save" => {
                            let state: State<AppState> = app.state();
                            let _ = state.power_manager.apply_governor(Governor::Powersave);
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
