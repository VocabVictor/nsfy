use crate::message::Message;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

const WRITE_QUEUE_CAPACITY: usize = 4096;
const MAX_COMMIT_BATCH: usize = 64;

struct WriteRequest {
    topic: String,
    msg: Message,
    keep: usize,
    done: oneshot::Sender<Result<(), String>>,
}

/// SQLite-backed message store. A single connection is sufficient because
/// SQLite serializes writes. `Persistence` owns the dedicated writer thread
/// that calls this synchronous API without blocking Tokio workers.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let journal_mode: String =
            conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        if path != ":memory:" && !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(rusqlite::Error::InvalidQuery);
        }
        conn.execute_batch(
            "PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;",
        )?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                topic    TEXT NOT NULL,
                id       TEXT NOT NULL,
                time     INTEGER NOT NULL,
                title    TEXT NOT NULL,
                message  TEXT NOT NULL,
                priority INTEGER NOT NULL,
                tags     TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT '[]',
                popup    INTEGER NOT NULL DEFAULT 0,
                bypass_dnd INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (topic, id)
            );
            CREATE INDEX IF NOT EXISTS idx_messages_topic_time ON messages(topic, time);",
        )?;
        let columns = {
            let mut stmt = conn.prepare("PRAGMA table_info(messages)")?;
            let columns: rusqlite::Result<Vec<String>> =
                stmt.query_map([], |row| row.get(1))?.collect();
            columns?
        };
        if !columns.iter().any(|name| name == "category") {
            conn.execute(
                "ALTER TABLE messages ADD COLUMN category TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }
        if !columns.iter().any(|name| name == "popup") {
            conn.execute(
                "ALTER TABLE messages ADD COLUMN popup INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
            conn.execute("UPDATE messages SET popup = priority >= 4", [])?;
        }
        if !columns.iter().any(|name| name == "bypass_dnd") {
            conn.execute(
                "ALTER TABLE messages ADD COLUMN bypass_dnd INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert one message, then prune that topic back down to `keep` rows —
    /// in one transaction, so the database is never briefly larger than the
    /// configured retention even under a crash between the two steps. This
    /// is what keeps history bounded per topic instead of growing forever.
    #[cfg(test)]
    pub fn insert_and_prune(
        &self,
        topic: &str,
        msg: &Message,
        keep: usize,
    ) -> rusqlite::Result<()> {
        self.insert_and_prune_batch(&[(topic, msg, keep)])
    }

    fn insert_and_prune_batch(&self, writes: &[(&str, &Message, usize)]) -> rusqlite::Result<()> {
        let mut conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let tx = conn.transaction()?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT OR REPLACE INTO messages
                 (topic, id, time, title, message, priority, tags, category, popup, bypass_dnd)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for (topic, msg, _) in writes {
                let tags = serde_json::to_string(&msg.tags).unwrap_or_default();
                let category = serde_json::to_string(&msg.category).unwrap_or_default();
                insert.execute(params![
                    topic,
                    msg.id,
                    msg.time,
                    msg.title,
                    msg.message,
                    msg.priority,
                    tags,
                    category,
                    msg.popup,
                    msg.bypass_dnd
                ])?;
            }
        }
        let retention: HashMap<&str, usize> = writes
            .iter()
            .map(|(topic, _, keep)| (*topic, *keep))
            .collect();
        {
            let mut prune = tx.prepare_cached(
                "DELETE FROM messages WHERE topic = ?1 AND id NOT IN (
                    SELECT id FROM messages WHERE topic = ?1 ORDER BY time DESC LIMIT ?2
                )",
            )?;
            for (topic, keep) in retention {
                prune.execute(params![topic, keep as i64])?;
            }
        }
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
                "SELECT id, time, title, message, priority, tags, category, popup, bypass_dnd FROM messages
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
                        category: {
                            let value: String = row.get(6)?;
                            serde_json::from_str(&value).unwrap_or_default()
                        },
                        popup: row.get(7)?,
                        bypass_dnd: row.get(8)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            rows.reverse(); // oldest first, matching the in-memory ring buffer order
            out.push((topic, rows));
        }
        Ok(out)
    }
}

#[derive(Clone)]
pub struct Persistence {
    sender: mpsc::Sender<WriteRequest>,
}

impl Persistence {
    pub fn sqlite(store: Arc<Store>) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel(WRITE_QUEUE_CAPACITY);
        std::thread::Builder::new()
            .name("nsfy-sqlite-writer".to_string())
            .spawn(move || writer_loop(store, receiver))
            .map_err(|error| format!("failed to start persistence writer: {error}"))?;
        Ok(Self { sender })
    }

    /// A publish is acknowledged only after its SQLite transaction commits.
    /// This keeps "200 OK" from meaning merely "queued for a best-effort
    /// background write" and guarantees replay after a successful response.
    pub async fn record(&self, topic: &str, msg: &Message, keep: usize) -> Result<(), String> {
        let (done, result) = oneshot::channel();
        self.sender
            .send(WriteRequest {
                topic: topic.to_string(),
                msg: msg.clone(),
                keep,
                done,
            })
            .await
            .map_err(|_| "persistence writer stopped".to_string())?;
        result
            .await
            .map_err(|_| "persistence writer stopped before commit".to_string())?
    }
}

fn writer_loop(store: Arc<Store>, mut receiver: mpsc::Receiver<WriteRequest>) {
    while let Some(first) = receiver.blocking_recv() {
        let mut batch = Vec::with_capacity(MAX_COMMIT_BATCH);
        batch.push(first);
        while batch.len() < MAX_COMMIT_BATCH {
            match receiver.try_recv() {
                Ok(request) => batch.push(request),
                Err(_) => break,
            }
        }
        let writes: Vec<_> = batch
            .iter()
            .map(|request| (request.topic.as_str(), &request.msg, request.keep))
            .collect();
        let outcome = store
            .insert_and_prune_batch(&writes)
            .map_err(|error| format!("failed to persist message batch: {error}"));
        for request in batch {
            let _ = request.done.send(outcome.clone());
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
