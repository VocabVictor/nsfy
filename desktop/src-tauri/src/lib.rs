use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WindowEvent};

pub mod cli;
pub mod config;
pub mod desktop_settings;
pub mod direct_http;
pub mod tray;

// --- App State ---

pub struct AppState {
    /// Set right before creating the notification popup window, and pulled
    /// once by that window's own frontend on mount — avoids any race with
    /// emitting an event to a window that isn't listening yet.
    pub pending_notification: Mutex<Option<Vec<PopupContent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupContent {
    pub title: String,
    pub body: String,
    pub time: i64,
    pub priority: u8,
}

// --- Tauri Commands ---

/// Compact notification center: a small, borderless, always-on-top window
/// at a corner (or center) of the screen listing the latest messages,
/// optionally closing itself after a few seconds. Separate from the main window
/// entirely, so it never requires bringing the whole app to the front.
#[tauri::command]
async fn show_notification_popup(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    messages: Vec<PopupContent>,
    position: String,
    persistent: bool,
) -> Result<(), String> {
    // A burst of messages shouldn't stack up multiple banners — replace
    // whichever one is still showing.
    if let Some(existing) = app.get_webview_window("notification-popup") {
        let _ = existing.close();
    }

    *state.pending_notification.lock().unwrap() = Some(messages);

    const WIDTH: f64 = 360.0;
    const HEIGHT: f64 = 300.0;
    const MARGIN: f64 = 16.0;
    // Extra clearance on the bottom edge so the banner doesn't land under
    // the Windows taskbar (or macOS Dock, if it's bottom-anchored there).
    const BOTTOM_CLEARANCE: f64 = 64.0;

    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no monitor found".to_string())?;
    let scale = monitor.scale_factor();
    let mon_size = monitor.size().to_logical::<f64>(scale);
    let mon_pos = monitor.position().to_logical::<f64>(scale);

    let (x, y) = match position.as_str() {
        "top-left" => (mon_pos.x + MARGIN, mon_pos.y + MARGIN),
        "top-right" => (
            mon_pos.x + mon_size.width - WIDTH - MARGIN,
            mon_pos.y + MARGIN,
        ),
        "bottom-left" => (
            mon_pos.x + MARGIN,
            mon_pos.y + mon_size.height - HEIGHT - BOTTOM_CLEARANCE,
        ),
        "bottom-right" => (
            mon_pos.x + mon_size.width - WIDTH - MARGIN,
            mon_pos.y + mon_size.height - HEIGHT - BOTTOM_CLEARANCE,
        ),
        _ => (
            mon_pos.x + (mon_size.width - WIDTH) / 2.0,
            mon_pos.y + (mon_size.height - HEIGHT) / 2.0,
        ),
    };

    let popup_builder = tauri::WebviewWindowBuilder::new(
        &app,
        "notification-popup",
        tauri::WebviewUrl::App("index.html?popup=1".into()),
    )
    .title("信鸽通知")
    .inner_size(WIDTH, HEIGHT)
    .position(x, y)
    .decorations(false);
    // macOS only exposes transparency through Tauri's private-API feature.
    // Keep the popup opaque there instead of depending on private system APIs.
    #[cfg(not(target_os = "macos"))]
    let popup_builder = popup_builder.transparent(true);

    let popup = popup_builder
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(true)
        .focused(false)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

    if !persistent {
        let popup_clone = popup.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let _ = popup_clone.close();
        });
    }

    Ok(())
}

/// Pulled once by the popup window's own frontend right after it mounts.
#[tauri::command]
fn get_pending_notification(state: tauri::State<'_, AppState>) -> Option<Vec<PopupContent>> {
    state.pending_notification.lock().unwrap().take()
}

/// Clicking the banner should bring the real app window forward, not just
/// dismiss the banner.
#[tauri::command]
fn focus_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.unminimize().map_err(|e| e.to_string())?;
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
    }
    if let Some(popup) = app.get_webview_window("notification-popup") {
        let _ = popup.close();
    }
    Ok(())
}

#[tauri::command]
fn load_shared_config() -> Result<Option<config::StoredConfig>, String> {
    config::load_existing()
}

#[tauri::command]
fn save_shared_config(app: AppHandle, config: config::StoredConfig) -> Result<(), String> {
    config::save(&config)?;
    tray::apply_dnd(&app, config.do_not_disturb);
    Ok(())
}

#[tauri::command]
fn apply_desktop_settings(
    app: AppHandle,
    auto_start: bool,
    start_minimized: bool,
    dnd_shortcut: String,
    show_shortcut: String,
) -> Result<(), String> {
    let preferences = desktop_settings::DesktopPreferences {
        auto_start,
        start_minimized,
        dnd_shortcut,
        show_shortcut,
    };
    desktop_settings::register_shortcuts(&app, &preferences)?;
    desktop_settings::apply_autostart(&app, auto_start)
}

#[tauri::command]
fn export_config(include_tokens: bool) -> Result<String, String> {
    let mut config = config::load()?;
    if !include_tokens {
        for server in &mut config.servers {
            server.token = None;
        }
    }
    serde_json::to_string_pretty(&config).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_config(content: String) -> Result<config::StoredConfig, String> {
    let config: config::StoredConfig =
        serde_json::from_str(&content).map_err(|error| format!("配置文件格式错误：{error}"))?;
    config::save(&config)?;
    Ok(config)
}

#[tauri::command]
fn reset_preferences() -> Result<(), String> {
    let mut current = config::load()?;
    let defaults = config::StoredConfig::default();
    current.popup_on_notify = defaults.popup_on_notify;
    current.notification_mode = defaults.notification_mode;
    current.popup_position = defaults.popup_position;
    current.layout_mode = defaults.layout_mode;
    current.window_behavior = defaults.window_behavior;
    current.do_not_disturb = defaults.do_not_disturb;
    current.dnd_allowed_priorities = defaults.dnd_allowed_priorities;
    current.advanced = defaults.advanced;
    config::save(&current)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_websocket::init())
        .manage(AppState {
            pending_notification: Mutex::new(None),
        })
        .manage(tray::TrayState::default())
        .invoke_handler(tauri::generate_handler![
            show_notification_popup,
            get_pending_notification,
            focus_main_window,
            load_shared_config,
            save_shared_config,
            apply_desktop_settings,
            export_config,
            import_config,
            reset_preferences,
            direct_http::post_state_direct,
            direct_http::post_message_direct,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            if event.state()
                                == tauri_plugin_global_shortcut::ShortcutState::Released
                            {
                                desktop_settings::handle_shortcut(app, shortcut);
                            }
                        })
                        .build(),
                )?;
                app.handle().plugin(tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    Some(vec!["--minimized"]),
                ))?;
                let config = config::load().unwrap_or_default();
                let preferences = desktop_settings::DesktopPreferences::from_config(&config);
                let _ = desktop_settings::register_shortcuts(app.handle(), &preferences);
                if desktop_settings::should_start_hidden(&config) {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }

            tray::setup(app)?;

            // Closing the window hides it instead of quitting — nsfy keeps
            // listening for notifications in the background, same as any
            // tray-resident app. Quit only via the tray menu.
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running nsfy desktop");
}
