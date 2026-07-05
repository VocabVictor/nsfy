use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A notification message published to a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub time: i64,
    #[serde(default)]
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Message {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7().to_string(),
            time: Utc::now().timestamp(),
            title: title.into(),
            message: message.into(),
            priority: 3,
            tags: Vec::new(),
        }
    }

    pub fn with_priority(mut self, p: u8) -> Self {
        self.priority = p.clamp(1, 5);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Incoming publish request body.
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    #[serde(default)]
    pub title: String,
    pub message: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_priority() -> u8 {
    3
}

impl From<PublishRequest> for Message {
    fn from(req: PublishRequest) -> Self {
        Message::new(req.title, req.message)
            .with_priority(req.priority)
            .with_tags(req.tags)
    }
}
