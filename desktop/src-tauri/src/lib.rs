use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WindowEvent,
};

pub mod cli;
pub mod config;

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
/// closing itself after a few seconds. Separate from the main window
/// entirely, so it never requires bringing the whole app to the front.
#[tauri::command]
async fn show_notification_popup(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    messages: Vec<PopupContent>,
    position: String,
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

    let popup = tauri::WebviewWindowBuilder::new(
        &app,
        "notification-popup",
        tauri::WebviewUrl::App("index.html?popup=1".into()),
    )
    .title("信鸽通知")
    .inner_size(WIDTH, HEIGHT)
    .position(x, y)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(true)
    .focused(false)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    let popup_clone = popup.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let _ = popup_clone.close();
    });

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
fn save_shared_config(config: config::StoredConfig) -> Result<(), String> {
    config::save(&config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            pending_notification: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            show_notification_popup,
            get_pending_notification,
            focus_main_window,
            load_shared_config,
            save_shared_config,
        ])
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "显示信鸽", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

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
