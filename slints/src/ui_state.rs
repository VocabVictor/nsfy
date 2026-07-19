use crate::config::TopicConfig;
use crate::notification;
use crate::websocket::{Event, IncomingMessage};
use crate::{AppWindow, MessageData, TopicData};
use chrono::{Local, TimeZone};
use slint::{ComponentHandle, Model, ModelExt, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone)]
pub struct UiState {
    topics: Rc<VecModel<TopicData>>,
    messages: Rc<VecModel<MessageData>>,
    notifications: bool,
    server: String,
}

impl UiState {
    pub fn new(ui: &AppWindow, topics: &[TopicConfig], server: &str, notifications: bool) -> Self {
        let topics = Rc::new(VecModel::from(
            topics
                .iter()
                .map(|topic| topic_data(topic, server))
                .collect::<Vec<_>>(),
        ));
        let messages = Rc::new(VecModel::default());
        ui.set_topics(ModelRc::from(topics.clone()));
        ui.set_messages(ModelRc::from(messages.clone()));
        if let Some(topic) = topics.row_data(0) {
            set_selected_messages(ui, messages.clone(), topic.name.to_string());
        }
        ui.set_server_address(server.into());
        ui.set_notifications_enabled(notifications);
        Self {
            topics,
            messages,
            notifications,
            server: server.to_owned(),
        }
    }

    pub fn start_event_pump(&self, ui: &AppWindow, receiver: mpsc::Receiver<Event>) -> Timer {
        let timer = Timer::default();
        let ui_weak = ui.as_weak();
        let state = self.clone();
        timer.start(TimerMode::Repeated, Duration::from_millis(75), move || {
            for event in receiver.try_iter() {
                state.apply(event);
            }
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_connection_summary(state.connection_summary().into());
            }
        });
        timer
    }

    pub fn add_topic(&self, topic: &TopicConfig) -> bool {
        if self.topic_index(&topic.key).is_some() {
            return false;
        }
        self.topics.push(topic_data(topic, &self.server));
        true
    }

    pub fn bind_topic_selection(&self, ui: &AppWindow) {
        let messages = self.messages.clone();
        let ui_weak = ui.as_weak();
        ui.on_topic_selected(move |label| {
            if let Some(ui) = ui_weak.upgrade() {
                set_selected_messages(&ui, messages.clone(), label.to_string());
            }
        });
    }

    fn apply(&self, event: Event) {
        match event {
            Event::Connected { key, connected } => self.set_connected(&key, connected),
            Event::Message {
                key,
                message,
                notify,
            } => {
                self.push_message(&key, &message);
                if self.notifications && notify {
                    let title = if message.title.is_empty() {
                        let label = self.topic_label(&key).unwrap_or_else(|| key.clone());
                        format!("信鸽 · {label}")
                    } else {
                        message.title.clone()
                    };
                    notification::show(title, message.message);
                }
            }
        }
    }

    fn set_connected(&self, key: &str, connected: bool) {
        let Some(index) = self.topic_index(key) else {
            return;
        };
        let Some(mut topic) = self.topics.row_data(index) else {
            return;
        };
        topic.online = connected;
        let status = if connected { "在线" } else { "重连中" };
        let server = topic.subtitle.split('·').next().unwrap_or_default().trim();
        topic.subtitle = format!("{server} · {status}").into();
        self.topics.set_row_data(index, topic);
    }

    fn push_message(&self, key: &str, message: &IncomingMessage) {
        let Some(index) = self.topic_index(key) else {
            return;
        };
        let Some(mut topic) = self.topics.row_data(index) else {
            return;
        };
        topic.unread = topic.unread.saturating_add(1);
        let label = topic.name.clone();
        self.topics.set_row_data(index, topic);
        self.messages.insert(
            0,
            MessageData {
                topic: label,
                title: message.title.as_str().into(),
                body: message.message.as_str().into(),
                time: format_time(message.time),
                priority: i32::from(message.priority),
                category: message.category.join(" › ").into(),
            },
        );
        if self.messages.row_count() > 500 {
            self.messages.remove(self.messages.row_count() - 1);
        }
    }

    fn topic_index(&self, key: &str) -> Option<usize> {
        (0..self.topics.row_count()).find(|&index| {
            self.topics
                .row_data(index)
                .is_some_and(|topic| topic.key.as_str() == key)
        })
    }

    fn topic_label(&self, key: &str) -> Option<String> {
        self.topic_index(key)
            .and_then(|index| self.topics.row_data(index))
            .map(|topic| topic.name.to_string())
    }

    fn connection_summary(&self) -> String {
        let connected = (0..self.topics.row_count())
            .filter(|&index| {
                self.topics
                    .row_data(index)
                    .is_some_and(|topic| topic.online)
            })
            .count();
        format!("WebSocket {connected}/{} 在线", self.topics.row_count())
    }
}

fn set_selected_messages(ui: &AppWindow, messages: Rc<VecModel<MessageData>>, label: String) {
    let filtered = messages.filter(move |message| message.topic.as_str() == label);
    ui.set_selected_messages(ModelRc::new(filtered));
}

fn topic_data(topic: &TopicConfig, server: &str) -> TopicData {
    TopicData {
        key: topic.key.as_str().into(),
        name: topic.label.as_str().into(),
        subtitle: format!("{} · 连接中", server_label(server)).into(),
        unread: 0,
        online: false,
        marker: topic.color,
    }
}

fn server_label(server: &str) -> &str {
    server
        .strip_prefix("https://")
        .or_else(|| server.strip_prefix("http://"))
        .unwrap_or(server)
}

fn format_time(timestamp: i64) -> SharedString {
    let now = Local::now();
    let Some(time) = Local.timestamp_opt(timestamp, 0).single() else {
        return "时间未知".into();
    };
    let minutes = now.signed_duration_since(time).num_minutes();
    match minutes {
        value if value < 1 => "刚刚".into(),
        value if value < 60 => format!("{value} 分钟前").into(),
        value if value < 1_440 => format!("{} 小时前", value / 60).into(),
        _ => time.format("%m月%d日 %H:%M").to_string().into(),
    }
}
