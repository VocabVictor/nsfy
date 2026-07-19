use crate::config::TopicConfig;
use futures_util::StreamExt;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc as tokio_mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{header::AUTHORIZATION, HeaderValue};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug)]
pub enum Command {
    Subscribe(TopicConfig),
}

#[derive(Debug)]
pub enum Event {
    Connected {
        key: String,
        connected: bool,
    },
    Message {
        key: String,
        message: IncomingMessage,
        notify: bool,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct IncomingMessage {
    pub id: String,
    pub time: i64,
    #[serde(default)]
    pub title: String,
    pub message: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub category: Vec<String>,
}

pub struct Client {
    commands: tokio_mpsc::UnboundedSender<Command>,
}

impl Client {
    pub fn start(server: String, token: Option<String>) -> (Self, mpsc::Receiver<Event>) {
        let (command_tx, mut command_rx) = tokio_mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::channel();

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("create WebSocket runtime");
            runtime.block_on(async move {
                while let Some(Command::Subscribe(topic)) = command_rx.recv().await {
                    let server = server.clone();
                    let token = token.clone();
                    let events = event_tx.clone();
                    tokio::spawn(subscribe_forever(server, token, topic, events));
                }
            });
        });

        (
            Self {
                commands: command_tx,
            },
            event_rx,
        )
    }

    pub fn subscribe(&self, topic: TopicConfig) {
        let _ = self.commands.send(Command::Subscribe(topic));
    }
}

async fn subscribe_forever(
    server: String,
    token: Option<String>,
    topic: TopicConfig,
    events: mpsc::Sender<Event>,
) {
    let url = websocket_url(&server, &topic.key);
    let mut seen = SeenMessages::default();
    let mut retry_secs = 2;
    let started_at = unix_time().saturating_sub(5);

    loop {
        let mut request = url
            .clone()
            .into_client_request()
            .expect("valid WebSocket URL");
        if let Some(token) = token.as_deref() {
            if let Ok(value) = HeaderValue::from_str(&format!("Bearer {token}")) {
                request.headers_mut().insert(AUTHORIZATION, value);
            }
        }
        let result = connect_async(request).await;
        if let Ok((mut socket, _)) = result {
            retry_secs = 2;
            if send_connection(&events, &topic.key, true).is_err() {
                return;
            }
            while let Some(frame) = socket.next().await {
                let text = match frame {
                    Ok(Message::Text(text)) => text,
                    Ok(Message::Close(_)) | Err(_) => break,
                    Ok(_) => continue,
                };
                let Ok(message) = serde_json::from_str::<IncomingMessage>(&text) else {
                    continue;
                };
                if !seen.insert(message.id.clone()) {
                    continue;
                }
                let notify = message.priority >= 4 && message.time >= started_at;
                if events
                    .send(Event::Message {
                        key: topic.key.clone(),
                        message,
                        notify,
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
        if send_connection(&events, &topic.key, false).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(retry_secs)).await;
        retry_secs = (retry_secs * 2).min(30);
    }
}

fn send_connection(events: &mpsc::Sender<Event>, key: &str, connected: bool) -> Result<(), ()> {
    events
        .send(Event::Connected {
            key: key.to_owned(),
            connected,
        })
        .map_err(|_| ())
}

pub fn websocket_url(server: &str, topic: &str) -> String {
    let base = server
        .trim_end_matches('/')
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);
    format!("{base}/{topic}/ws")
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Default)]
struct SeenMessages {
    ids: HashSet<String>,
    order: VecDeque<String>,
}

impl SeenMessages {
    fn insert(&mut self, id: String) -> bool {
        if !self.ids.insert(id.clone()) {
            return false;
        }
        self.order.push_back(id);
        if self.order.len() > 1_000 {
            if let Some(oldest) = self.order.pop_front() {
                self.ids.remove(&oldest);
            }
        }
        true
    }
}

fn default_priority() -> u8 {
    3
}

#[cfg(test)]
#[path = "../tests/unit/websocket.rs"]
mod tests;
