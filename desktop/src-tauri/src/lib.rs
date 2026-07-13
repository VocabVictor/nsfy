use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WindowEvent,
};
use tokio_tungstenite::connect_async;
use futures_util::StreamExt;

// --- Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsfyMessage {
    pub id: String,
    pub time: i64,
    pub title: String,
    pub message: String,
    pub priority: u8,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WsMessageEvent {
    pub topic: String,
    pub server: String,
    pub message: NsfyMessage,
}

// --- App State ---

pub struct AppState {
    pub servers: Mutex<Vec<ServerConfig>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: String,
    pub name: String,
}

// --- Tauri Commands ---

#[tauri::command]
async fn publish_message(
    server: String,
    topic: String,
    title: String,
    message: String,
    priority: u8,
    tags: Vec<String>,
) -> Result<String, String> {
    let body = serde_json::json!({
        "title": title,
        "message": message,
        "priority": priority,
        "tags": tags,
    });
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/{}", server, topic))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if res.status().is_success() {
        Ok("published".into())
    } else {
        Err(format!("server returned {}", res.status()))
    }
}

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
async fn check_health(server: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(&server)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let body = res.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
fn start_ws(
    app: AppHandle,
    server: String,
    topic: String,
) -> Result<(), String> {
    let app_clone = app.clone();
    let server_clone = server.clone();
    let topic_clone = topic.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let ws_url = server_clone
                .replace("http://", "ws://")
                .replace("https://", "wss://");
            let url = format!("{}/{}/ws", ws_url, topic_clone);

            loop {
                match connect_async(&url).await {
                    Ok((mut ws, _)) => {
                        let _ = app_clone.emit("ws-connected", &topic_clone);

                        while let Some(Ok(msg)) = ws.next().await {
                            if let Ok(text) = msg.to_text() {
                                if let Ok(nsfy_msg) = serde_json::from_str::<NsfyMessage>(text) {
                                    let _ = app_clone.emit(
                                        "ws-message",
                                        WsMessageEvent {
                                            topic: topic_clone.clone(),
                                            server: server_clone.clone(),
                                            message: nsfy_msg.clone(),
                                        },
                                    );

                                    // Native notification for high priority
                                    if nsfy_msg.priority >= 4 {
                                        use tauri_plugin_notification::NotificationExt;
                                        let title = if nsfy_msg.title.is_empty() {
                                            topic_clone.clone()
                                        } else {
                                            nsfy_msg.title.clone()
                                        };
                                        let _ = app_clone
                                            .notification()
                                            .builder()
                                            .title(title)
                                            .body(nsfy_msg.message.clone())
                                            .show();
                                    }
                                }
                            }
                        }
                        // ws.next() returned None or an error — connection closed.
                    }
                    Err(e) => {
                        eprintln!("ws connect error for {}: {}", topic_clone, e);
                    }
                }

                let _ = app_clone.emit("ws-disconnected", &topic_clone);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            servers: Mutex::new(Vec::new()),
            pending_notification: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            publish_message,
            check_health,
            start_ws,
            show_notification_popup,
            get_pending_notification,
            focus_main_window,
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
