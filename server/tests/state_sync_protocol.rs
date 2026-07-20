mod common;

use common::{message, TestServer, TOKEN};
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::{header::AUTHORIZATION, HeaderValue};
use tungstenite::{connect, stream::MaybeTlsStream};

fn state_request(id: &str, status: &str) -> Value {
    serde_json::json!({ "updates": [{ "id": id, "status": status }] })
}

fn prepare_topic(server: &TestServer) {
    server
        .request(Client::new().post(format!("{}/alerts", server.base_url)))
        .json(&message("state sync"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
}

fn state_socket(
    server: &TestServer,
) -> tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>> {
    let mut request = format!("{}/alerts/state/ws", server.ws_url)
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {TOKEN}")).unwrap(),
    );
    connect(request).unwrap().0
}

#[test]
fn state_snapshot_persists_latest_message_status() {
    let server = TestServer::fresh();
    prepare_topic(&server);
    let client = Client::new();
    for status in ["read", "trash"] {
        server
            .request(client.post(format!("{}/alerts/state", server.base_url)))
            .json(&state_request("message-one", status))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let mut socket = state_socket(&server);
    let payload: Value =
        serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
    assert_eq!(payload["type"], "snapshot");
    assert_eq!(payload["updates"].as_array().unwrap().len(), 1);
    assert_eq!(payload["updates"][0]["status"], "trash");
}

#[test]
fn state_websocket_broadcasts_updates_immediately() {
    let server = TestServer::fresh();
    prepare_topic(&server);
    let mut socket = state_socket(&server);
    let snapshot: Value =
        serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
    assert!(snapshot["updates"].as_array().unwrap().is_empty());
    server
        .request(Client::new().post(format!("{}/alerts/state", server.base_url)))
        .json(&state_request("message-two", "read"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
    }
    let payload: Value =
        serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
    assert_eq!(payload["type"], "state");
    assert_eq!(payload["updates"][0]["id"], "message-two");
}

#[test]
fn state_updates_require_write_permission_and_validate_batches() {
    let server = TestServer::fresh();
    prepare_topic(&server);
    let missing = Client::new()
        .post(format!("{}/alerts/state", server.base_url))
        .json(&state_request("message", "read"))
        .send()
        .unwrap();
    assert_eq!(missing.status(), 401);
    let empty = server
        .request(Client::new().post(format!("{}/alerts/state", server.base_url)))
        .json(&serde_json::json!({ "updates": [] }))
        .send()
        .unwrap();
    assert_eq!(empty.status(), 400);
}
