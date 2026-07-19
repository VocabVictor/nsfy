mod common;

use common::{message, TestServer};
use reqwest::blocking::Client;
use std::fs;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::{header::AUTHORIZATION, HeaderValue};

#[test]
fn topic_read_and_write_tokens_are_independent() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("nsfy.db");
    let access = temp.path().join("access.json");
    fs::write(
        &access,
        r#"{
            "default": "deny",
            "topics": {
                "alerts": { "read": "reader", "write": "writer" }
            }
        }"#,
    )
    .unwrap();
    let access_arg = access.to_str().unwrap();
    let server = TestServer::spawn(&db, &["--topic-access-file", access_arg]);
    let client = Client::new();

    let reader_publish = client
        .post(format!("{}/alerts", server.base_url))
        .bearer_auth("reader")
        .json(&message("denied"))
        .send()
        .unwrap();
    assert_eq!(reader_publish.status(), 401);
    client
        .post(format!("{}/alerts", server.base_url))
        .bearer_auth("writer")
        .json(&message("allowed"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    let messages: Vec<serde_json::Value> = client
        .get(format!("{}/alerts/json", server.base_url))
        .bearer_auth("reader")
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(messages[0]["message"], "allowed");
    let unknown = client
        .get(format!("{}/other/json", server.base_url))
        .bearer_auth("reader")
        .send()
        .unwrap();
    assert_eq!(unknown.status(), 401);
    server
        .request(client.get(format!("{}/", server.base_url)))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[test]
fn websocket_read_token_can_receive_but_write_token_cannot() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("nsfy.db");
    let access = temp.path().join("access.json");
    fs::write(
        &access,
        r#"{"topics":{"alerts":{"read":"reader","write":"writer"}}}"#,
    )
    .unwrap();
    let server = TestServer::spawn(&db, &["--topic-access-file", access.to_str().unwrap()]);
    let mut reader = format!("{}/alerts/ws", server.ws_url)
        .into_client_request()
        .unwrap();
    reader
        .headers_mut()
        .insert(AUTHORIZATION, HeaderValue::from_static("Bearer reader"));
    assert!(tungstenite::connect(reader).is_ok());
    let mut writer = format!("{}/alerts/ws", server.ws_url)
        .into_client_request()
        .unwrap();
    writer
        .headers_mut()
        .insert(AUTHORIZATION, HeaderValue::from_static("Bearer writer"));
    let (mut socket, _) = tungstenite::connect(writer).unwrap();
    assert!(socket.read().unwrap().is_close());
}
