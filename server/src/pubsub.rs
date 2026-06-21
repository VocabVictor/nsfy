use crate::message::Message;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, trace};

/// A single topic with broadcast channel and message cache.
pub struct Topic {
    pub tx: broadcast::Sender<Arc<Message>>,
    cache: tokio::sync::Mutex<VecDeque<Arc<Message>>>,
    max_cache: usize,
    subscriber_count: AtomicU64,
}

impl Topic {
    pub fn new(max_cache: usize) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            tx,
            cache: tokio::sync::Mutex::new(VecDeque::with_capacity(max_cache)),
            max_cache,
            subscriber_count: AtomicU64::new(0),
        }
    }

    /// Publish a message: push to cache, broadcast to subscribers.
    pub async fn publish(&self, msg: Message) {
        let msg = Arc::new(msg);
        // Push to ring buffer
        {
            let mut cache = self.cache.lock().await;
            if cache.len() >= self.max_cache {
                cache.pop_front();
            }
            cache.push_back(Arc::clone(&msg));
        }
        // Broadcast — ignore errors (no subscribers)
        let _ = self.tx.send(msg);
    }

    /// Get cached messages since a given ID (for HTTP poll).
    pub async fn messages_since(&self, since_id: Option<&str>) -> Vec<Arc<Message>> {
        let cache = self.cache.lock().await;
        if let Some(sid) = since_id {
            // Find the position after the given ID
            let pos = cache.iter().position(|m| m.id == sid);
            match pos {
                Some(idx) => cache.iter().skip(idx + 1).cloned().collect(),
                None => cache.iter().cloned().collect(), // ID not found, return all
            }
        } else {
            cache.iter().cloned().collect()
        }
    }

    /// Return all cached messages as owned values.
    #[allow(dead_code)]
    pub async fn all_messages_owned(&self) -> Vec<Message> {
        let cache = self.cache.lock().await;
        cache.iter().map(|m| (**m).clone()).collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Message>> {
        self.subscriber_count.fetch_add(1, Ordering::Relaxed);
        self.tx.subscribe()
    }

    pub fn subscriber_count(&self) -> u64 {
        self.subscriber_count.load(Ordering::Relaxed)
    }
}

/// Global topic registry.
pub struct PubSub {
    topics: DashMap<String, Arc<Topic>>,
    cache_size: usize,
}

impl PubSub {
    pub fn new(cache_size: usize) -> Self {
        info!("pubsub engine initialized, cache_size={}", cache_size);
        Self {
            topics: DashMap::new(),
            cache_size,
        }
    }

    /// Get or create a topic.
    pub fn get_or_create(&self, name: &str) -> Arc<Topic> {
        // Fast path: already exists
        if let Some(topic) = self.topics.get(name) {
            return Arc::clone(&topic);
        }
        // Slow path: create
        let topic = Arc::new(Topic::new(self.cache_size));
        self.topics.insert(name.to_string(), Arc::clone(&topic));
        debug!("topic created: {}", name);
        topic
    }

    /// Publish a message to a topic (creates topic if needed).
    pub async fn publish(&self, topic: &str, msg: Message) {
        let t = self.get_or_create(topic);
        t.publish(msg).await;
        trace!("published to topic: {}", topic);
    }

    /// Subscribe to a topic, first receiving cached messages, then live.
    pub fn subscribe(&self, topic: &str) -> (Vec<Arc<Message>>, broadcast::Receiver<Arc<Message>>) {
        let t = self.get_or_create(topic);
        // We can't call async all_messages in sync context easily,
        // so return an empty vec and let caller handle cache separately.
        let rx = t.subscribe();
        (Vec::new(), rx)
    }

    /// Get cached messages for a topic (owned).
    pub async fn get_messages(&self, topic: &str, since_id: Option<&str>) -> Vec<Message> {
        let t = self.get_or_create(topic);
        let arcs = t.messages_since(since_id).await;
        arcs.iter().map(|m| (**m).clone()).collect()
    }

    /// Get cached messages as Arc for internal use (WS broadcast).
    pub async fn get_messages_arc(&self, topic: &str, since_id: Option<&str>) -> Vec<Arc<Message>> {
        let t = self.get_or_create(topic);
        t.messages_since(since_id).await
    }

    pub fn stats(&self) -> PubSubStats {
        let topic_count = self.topics.len();
        let total_subscribers: u64 = self.topics.iter().map(|t| t.subscriber_count()).sum();
        let topic_names: Vec<String> = self.topics
            .iter()
            .map(|t| t.key().clone())
            .collect();

        PubSubStats {
            topics: topic_count,
            total_subscribers,
            topic_names,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PubSubStats {
    pub topics: usize,
    pub total_subscribers: u64,
    pub topic_names: Vec<String>,
}
