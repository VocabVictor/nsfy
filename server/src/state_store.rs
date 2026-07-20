use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_BATCH: usize = 500;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Read,
    Trash,
    Purged,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StateChange {
    pub id: String,
    pub status: MessageStatus,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct StateRequest {
    pub updates: Vec<StateChange>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateEvent {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub updates: Vec<StateChange>,
}

pub struct StateStore {
    conn: Mutex<Connection>,
}

impl StateStore {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS message_states (
                topic TEXT NOT NULL,
                id TEXT NOT NULL,
                status TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (topic, id)
             );
             CREATE INDEX IF NOT EXISTS idx_message_states_topic_updated
                ON message_states(topic, updated_at);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn load(&self, topic: &str, limit: usize) -> rusqlite::Result<Vec<StateChange>> {
        let conn = self.conn.lock().unwrap_or_else(|lock| lock.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, status, updated_at FROM message_states
             WHERE topic = ?1 ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![topic, limit as i64], |row| {
            let status: String = row.get(1)?;
            Ok(StateChange {
                id: row.get(0)?,
                status: parse_status(&status)?,
                updated_at: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn record(
        &self,
        topic: &str,
        updates: &[StateChange],
        keep: usize,
    ) -> rusqlite::Result<Vec<StateChange>> {
        if updates.is_empty() || updates.len() > MAX_BATCH {
            return Err(rusqlite::Error::InvalidParameterCount(
                updates.len(),
                MAX_BATCH,
            ));
        }
        let now = unix_millis();
        let stamped: Vec<_> = updates
            .iter()
            .enumerate()
            .map(|(index, update)| StateChange {
                id: update.id.clone(),
                status: update.status,
                updated_at: now.saturating_add(index as i64),
            })
            .collect();
        let mut conn = self.conn.lock().unwrap_or_else(|lock| lock.into_inner());
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO message_states(topic, id, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(topic, id) DO UPDATE SET
                    status = excluded.status, updated_at = excluded.updated_at",
            )?;
            for update in &stamped {
                stmt.execute(params![
                    topic,
                    update.id,
                    status_name(update.status),
                    update.updated_at
                ])?;
            }
        }
        tx.execute(
            "DELETE FROM message_states WHERE topic = ?1 AND id NOT IN (
                SELECT id FROM message_states WHERE topic = ?1
                ORDER BY updated_at DESC LIMIT ?2
             )",
            params![topic, keep as i64],
        )?;
        tx.commit()?;
        Ok(stamped)
    }
}

pub fn validate_request(request: &StateRequest) -> Result<(), &'static str> {
    if request.updates.is_empty() || request.updates.len() > MAX_BATCH {
        return Err("state update batch must contain 1 to 500 items");
    }
    if request.updates.iter().any(|update| {
        update.id.is_empty() || update.id.len() > 128 || update.id.chars().any(char::is_control)
    }) {
        return Err("invalid message id");
    }
    Ok(())
}

fn status_name(status: MessageStatus) -> &'static str {
    match status {
        MessageStatus::Read => "read",
        MessageStatus::Trash => "trash",
        MessageStatus::Purged => "purged",
    }
}

fn parse_status(value: &str) -> rusqlite::Result<MessageStatus> {
    match value {
        "read" => Ok(MessageStatus::Read),
        "trash" => Ok(MessageStatus::Trash),
        "purged" => Ok(MessageStatus::Purged),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
#[path = "../tests/unit/state_store.rs"]
mod tests;
