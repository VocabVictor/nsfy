#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod notification;
mod ui_state;
mod websocket;

use config::{topic_from_key, AppConfig};
use slint::{CloseRequestResponse, ComponentHandle};
use ui_state::UiState;
use websocket::Client;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load().map_err(std::io::Error::other)?;
    let ui = AppWindow::new()?;
    let tray = NsfyTray::new()?;
    let (client, events) = Client::start(config.server.clone(), config.token.clone());
    let state = UiState::new(&ui, &config.topics, &config.server, config.notifications);
    let _event_timer = state.start_event_pump(&ui, events);
    state.bind_topic_selection(&ui);

    for topic in &config.topics {
        client.subscribe(topic.clone());
    }
    wire_window_and_tray(&ui, &tray);
    wire_new_subscriptions(&ui, client, state);

    ui.show()?;
    tray.show()?;
    Ok(slint::run_event_loop()?)
}

fn wire_window_and_tray(ui: &AppWindow, tray: &NsfyTray) {
    ui.window()
        .on_close_requested(|| CloseRequestResponse::HideWindow);

    let window = ui.as_weak();
    tray.on_clicked_requested(move || show_window(&window));
    let window = ui.as_weak();
    tray.on_open_requested(move || show_window(&window));
    tray.on_quit_requested(|| {
        let _ = slint::quit_event_loop();
    });
}

fn show_window(window: &slint::Weak<AppWindow>) {
    if let Some(window) = window.upgrade() {
        let _ = window.show();
        window.window().request_redraw();
    }
}

fn wire_new_subscriptions(ui: &AppWindow, client: Client, state: UiState) {
    ui.on_subscribe_requested(move |key| {
        let Some(topic) = topic_from_key(key.as_str()) else {
            return;
        };
        if !state.add_topic(&topic) {
            return;
        }
        client.subscribe(topic);
    });
}
