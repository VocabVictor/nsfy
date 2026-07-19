use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager,
};

use crate::config;

const TRAY_ID: &str = "main";

#[derive(Default)]
pub struct TrayState {
    dnd_item: Mutex<Option<CheckMenuItem<tauri::Wry>>>,
}

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let dnd_enabled = config::load()
        .map(|settings| settings.do_not_disturb)
        .unwrap_or(false);
    let show_item = MenuItem::with_id(app, "show", "显示信鸽", true, None::<&str>)?;
    let dnd_item = CheckMenuItem::with_id(
        app,
        "dnd",
        "勿扰模式（Ctrl+Alt+D）",
        true,
        dnd_enabled,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &dnd_item, &quit_item])?;

    *app.state::<TrayState>().dnd_item.lock().unwrap() = Some(dnd_item);
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_image(app.app_handle(), dnd_enabled))
        .tooltip(tray_tooltip(dnd_enabled))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "dnd" => toggle_dnd(app),
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
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    apply_dnd(app.app_handle(), dnd_enabled);
    Ok(())
}

pub fn toggle_dnd(app: &AppHandle) {
    if let Ok(mut settings) = config::load() {
        settings.do_not_disturb = !settings.do_not_disturb;
        if config::save(&settings).is_ok() {
            apply_dnd(app, settings.do_not_disturb);
            sync_frontend(app, settings.do_not_disturb);
        }
    }
}

pub fn apply_dnd(app: &AppHandle, enabled: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_image(app, enabled)));
        let _ = tray.set_tooltip(Some(tray_tooltip(enabled)));
    }
    if let Some(item) = app.state::<TrayState>().dnd_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(enabled);
        let text = if enabled {
            "勿扰模式：已开启（Ctrl+Alt+D）"
        } else {
            "勿扰模式（Ctrl+Alt+D）"
        };
        let _ = item.set_text(text);
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(enabled);
        if enabled {
            let _ = window.hide();
        }
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn sync_frontend(app: &AppHandle, enabled: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let script = format!(
            "window.dispatchEvent(new CustomEvent('nsfy-dnd-changed',{{detail:{enabled}}}))"
        );
        let _ = window.eval(&script);
    }
}

fn tray_tooltip(enabled: bool) -> &'static str {
    if enabled {
        "信鸽 · 勿扰模式"
    } else {
        "信鸽 · 正常接收"
    }
}

fn tray_image(app: &AppHandle, enabled: bool) -> Image<'static> {
    let image = app.default_window_icon().unwrap().clone().to_owned();
    if enabled {
        dnd_image(&image)
    } else {
        image
    }
}

fn dnd_image(source: &Image<'_>) -> Image<'static> {
    let width = source.width();
    let height = source.height();
    let mut rgba = source.rgba().to_vec();
    if width == 0 || height == 0 {
        return Image::new_owned(rgba, width, height);
    }
    for pixel in rgba.chunks_exact_mut(4) {
        let gray = ((u16::from(pixel[0]) * 30
            + u16::from(pixel[1]) * 59
            + u16::from(pixel[2]) * 11)
            / 100) as u8;
        pixel[0] = gray;
        pixel[1] = gray;
        pixel[2] = gray;
    }
    let thickness = (width.min(height) / 12).max(1);
    for y in 0..height {
        let line_x = width.saturating_sub(1) - y * width / height.max(1);
        for x in line_x.saturating_sub(thickness)..=(line_x + thickness).min(width - 1) {
            let offset = ((y * width + x) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&[239, 68, 68, 255]);
        }
    }
    Image::new_owned(rgba, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dnd_icon_is_grayscale_with_a_red_slash() {
        let source = Image::new_owned([40, 120, 220, 255].repeat(16), 4, 4);
        let changed = dnd_image(&source);
        assert!(changed
            .rgba()
            .chunks_exact(4)
            .any(|pixel| pixel == [239, 68, 68, 255]));
        assert!(changed
            .rgba()
            .chunks_exact(4)
            .any(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2]));
    }
}
