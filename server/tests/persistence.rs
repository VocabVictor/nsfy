mod common;

use common::{message, TestServer};
use reqwest::blocking::Client;
use rusqlite::Connection;

#[test]
fn acknowledged_messages_survive_a_restart() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("messages.db");
    let first = TestServer::spawn(&db, &[]);
    first
        .request(Client::new().post(format!("{}/alerts", first.base_url)))
        .json(&message("durable"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    first.stop();

    let second = TestServer::spawn(&db, &[]);
    let messages: Vec<serde_json::Value> = second
        .request(Client::new().get(format!("{}/alerts/json", second.base_url)))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["message"], "durable");
}

#[test]
fn legacy_database_is_migrated_without_losing_messages() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("legacy.db");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE messages (
                topic TEXT NOT NULL, id TEXT NOT NULL, time INTEGER NOT NULL,
                title TEXT NOT NULL, message TEXT NOT NULL, priority INTEGER NOT NULL,
                tags TEXT NOT NULL, PRIMARY KEY(topic, id)
             );
             INSERT INTO messages VALUES
                ('alerts','legacy-id',1,'old','kept',3,'[]');",
        )
        .unwrap();
    drop(connection);
    let server = TestServer::spawn(&db, &[]);
    let messages: Vec<serde_json::Value> = server
        .request(Client::new().get(format!("{}/alerts/json", server.base_url)))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(messages[0]["message"], "kept");
    assert_eq!(messages[0]["category"], serde_json::json!([]));
    assert_eq!(messages[0]["popup"], false);
    assert_eq!(messages[0]["bypassDnd"], false);
}

#[test]
fn sqlite_runs_in_wal_full_synchronous_mode() {
    let server = TestServer::fresh();
    let connection = Connection::open(&server.db_path).unwrap();
    let journal: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal.to_lowercase(), "wal");
    assert_eq!(synchronous, 2);
}
