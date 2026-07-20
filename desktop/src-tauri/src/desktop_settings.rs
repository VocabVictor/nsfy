use serde_json::Value;
use std::str::FromStr;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{config, tray};

#[derive(Clone)]
pub struct DesktopPreferences {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub dnd_shortcut: String,
    pub show_shortcut: String,
}

impl DesktopPreferences {
    pub fn from_config(config: &config::StoredConfig) -> Self {
        let advanced = &config.advanced;
        Self {
            auto_start: boolean(advanced, "autoStart", false),
            start_minimized: boolean(advanced, "startMinimized", true),
            dnd_shortcut: text(advanced, "dndShortcut", "Ctrl+Alt+D"),
            show_shortcut: text(advanced, "showShortcut", "Ctrl+Alt+N"),
        }
    }
}

pub fn register_shortcuts(app: &AppHandle, preferences: &DesktopPreferences) -> Result<(), String> {
    let dnd = parse_shortcut(&preferences.dnd_shortcut)?;
    let show = parse_shortcut(&preferences.show_shortcut)?;
    if dnd == show {
        return Err("两个快捷键不能相同".into());
    }
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|error| error.to_string())?;
    if let Err(error) = manager.register_multiple([dnd, show]) {
        let _ = manager.unregister_all();
        return Err(format!("快捷键已被其他程序占用或格式无效：{error}"));
    }
    Ok(())
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| error.to_string())
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let preferences = config::load()
        .map(|value| DesktopPreferences::from_config(&value))
        .unwrap_or_else(|_| DesktopPreferences {
            auto_start: false,
            start_minimized: true,
            dnd_shortcut: "Ctrl+Alt+D".into(),
            show_shortcut: "Ctrl+Alt+N".into(),
        });
    if parse_shortcut(&preferences.dnd_shortcut).ok().as_ref() == Some(shortcut) {
        tray::toggle_dnd(app);
    } else if parse_shortcut(&preferences.show_shortcut).ok().as_ref() == Some(shortcut) {
        tray::show_main_window(app);
    }
}

pub fn should_start_hidden(config: &config::StoredConfig) -> bool {
    std::env::args().any(|argument| argument == "--minimized")
        && DesktopPreferences::from_config(config).start_minimized
}

fn parse_shortcut(value: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(value.trim()).map_err(|error| error.to_string())
}

fn boolean(value: &Value, key: &str, fallback: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

fn text(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
#[path = "../tests/unit/desktop_settings.rs"]
mod tests;
