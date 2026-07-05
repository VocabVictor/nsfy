use crate::message::Message;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

/// SQLite-backed message store. Plain synchronous rusqlite calls — callers
/// (see `Persistence::record` below) push the actual I/O onto a blocking
/// thread pool via `spawn_blocking` rather than this module doing it
/// itself, since a `&self` method can't hand a borrowed connection into a
/// `'static` blocking task anyway; the caller already holds an owned `Arc`.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                topic    TEXT NOT NULL,
                id       TEXT NOT NULL,
                time     INTEGER NOT NULL,
                title    TEXT NOT NULL,
                message  TEXT NOT NULL,
                priority INTEGER NOT NULL,
                tags     TEXT NOT NULL,
                PRIMARY KEY (topic, id)
            );
            CREATE INDEX IF NOT EXISTS idx_messages_topic_time ON messages(topic, time);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert one message, then prune that topic back down to `keep` rows —
    /// in one transaction, so the database is never briefly larger than the
    /// configured retention even under a crash between the two steps. This
    /// is what keeps history bounded per topic instead of growing forever.
    pub fn insert_and_prune(
        &self,
        topic: &str,
        msg: &Message,
        keep: usize,
    ) -> rusqlite::Result<()> {
        let tags = serde_json::to_string(&msg.tags).unwrap_or_default();
        let mut conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO messages (topic, id, time, title, message, priority, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                topic,
                msg.id,
                msg.time,
                msg.title,
                msg.message,
                msg.priority,
                tags
            ],
        )?;
        tx.execute(
            "DELETE FROM messages WHERE topic = ?1 AND id NOT IN (
                SELECT id FROM messages WHERE topic = ?1 ORDER BY time DESC LIMIT ?2
            )",
            params![topic, keep as i64],
        )?;
        tx.commit()
    }

    /// Load every topic's most recent `keep` messages back, oldest first —
    /// used once at startup to replay history into the in-memory cache.
    pub fn load_all(&self, keep: usize) -> rusqlite::Result<Vec<(String, Vec<Message>)>> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let topics: Vec<String> = {
            let mut stmt = conn.prepare("SELECT DISTINCT topic FROM messages")?;
            let rows: rusqlite::Result<Vec<String>> =
                stmt.query_map([], |row| row.get(0))?.collect();
            rows?
        };

        let mut out = Vec::with_capacity(topics.len());
        for topic in topics {
            let mut stmt = conn.prepare(
                "SELECT id, time, title, message, priority, tags FROM messages
                 WHERE topic = ?1 ORDER BY time DESC LIMIT ?2",
            )?;
            let mut rows: Vec<Message> = stmt
                .query_map(params![topic, keep as i64], |row| {
                    let tags_json: String = row.get(5)?;
                    Ok(Message {
                        id: row.get(0)?,
                        time: row.get(1)?,
                        title: row.get(2)?,
                        message: row.get(3)?,
                        priority: row.get(4)?,
                        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            rows.reverse(); // oldest first, matching the in-memory ring buffer order
            out.push((topic, rows));
        }
        Ok(out)
    }
}

/// Message persistence backend. `None` means pure in-memory (no `--db-path`
/// given) — the SQLite support is always compiled in, but only activates
/// when the operator points it at a file.
pub enum Persistence {
    None,
    Sqlite(Arc<Store>),
}

impl Persistence {
    /// Fire-and-forget: this never adds latency to, or can fail, the
    /// publish response. A write failure is logged and dropped — the
    /// message was published either way, this is purely about surviving a
    /// restart, not about whether the publish itself succeeded.
    pub fn record(&self, topic: &str, msg: &Message, keep: usize) {
        match self {
            Persistence::None => {}
            Persistence::Sqlite(store) => {
                let store = Arc::clone(store);
                let topic = topic.to_string();
                let msg = msg.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        store.insert_and_prune(&topic, &msg, keep)
                    })
                    .await;
                    match result {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => tracing::warn!("failed to persist message: {}", e),
                        Err(e) => tracing::warn!("persistence task panicked: {}", e),
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

    fn msg(id: &str, time: i64, body: &str) -> Message {
        Message {
            id: id.to_string(),
            time,
            title: "t".to_string(),
            message: body.to_string(),
            priority: 3,
            tags: vec!["tag1".to_string()],
        }
    }

    #[test]
    fn insert_and_prune_keeps_only_the_most_recent_n_per_topic() {
        let store = Store::open(":memory:").unwrap();
        for i in 0..5 {
            store
                .insert_and_prune("alerts", &msg(&format!("id{i}"), i, "body"), 3)
                .unwrap();
        }
        let loaded = store.load_all(10).unwrap();
        assert_eq!(loaded.len(), 1);
        let (topic, msgs) = &loaded[0];
        assert_eq!(topic, "alerts");
        assert_eq!(msgs.len(), 3, "should have pruned down to the keep limit");
        // Oldest-first order, and it's the 3 most recent (id2, id3, id4).
        assert_eq!(msgs[0].id, "id2");
        assert_eq!(msgs[2].id, "id4");
    }

    #[test]
    fn load_all_round_trips_fields_including_tags() {
        let store = Store::open(":memory:").unwrap();
        store
            .insert_and_prune("t", &msg("a", 1, "hello"), 10)
            .unwrap();
        let loaded = store.load_all(10).unwrap();
        let (_, msgs) = &loaded[0];
        assert_eq!(msgs[0].message, "hello");
        assert_eq!(msgs[0].tags, vec!["tag1".to_string()]);
    }

    #[test]
    fn separate_topics_are_pruned_independently() {
        let store = Store::open(":memory:").unwrap();
        for i in 0..3 {
            store
                .insert_and_prune("a", &msg(&format!("a{i}"), i, "x"), 1)
                .unwrap();
            store
                .insert_and_prune("b", &msg(&format!("b{i}"), i, "x"), 5)
                .unwrap();
        }
        let loaded: std::collections::HashMap<_, _> =
            store.load_all(10).unwrap().into_iter().collect();
        assert_eq!(loaded["a"].len(), 1, "topic a keeps only 1");
        assert_eq!(
            loaded["b"].len(),
            3,
            "topic b keeps up to 5, only 3 were written"
        );
    }
}
