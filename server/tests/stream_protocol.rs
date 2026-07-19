mod common;

use common::{message, TestServer, TOKEN};
use reqwest::blocking::Client;
use std::io::{BufRead, BufReader};
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::{header::AUTHORIZATION, HeaderValue};
use tungstenite::{connect, Message};

#[test]
fn websocket_authenticates_before_replaying_messages() {
    let server = TestServer::fresh();
    let client = Client::new();
    server
        .request(client.post(format!("{}/alerts", server.base_url)))
        .json(&message("cached"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();

    let (mut socket, _) = connect(format!("{}/alerts/ws", server.ws_url)).unwrap();
    socket
        .send(Message::Text(
            serde_json::json!({ "type": "auth", "token": TOKEN })
                .to_string()
                .into(),
        ))
        .unwrap();
    let payload = socket.read().unwrap().into_text().unwrap();
    let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(value["message"], "cached");
}

#[test]
fn websocket_accepts_native_authorization_headers_and_is_read_only() {
    let server = TestServer::fresh();
    let mut request = format!("{}/alerts/ws", server.ws_url)
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {TOKEN}")).unwrap(),
    );
    let (mut socket, _) = connect(request).unwrap();
    socket
        .send(Message::Text(
            message("must not publish").to_string().into(),
        ))
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let messages: Vec<serde_json::Value> = server
        .request(Client::new().get(format!("{}/alerts/json", server.base_url)))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert!(messages.is_empty());
    socket.close(None).unwrap();
}

#[test]
fn server_sent_events_replay_cached_messages() {
    let server = TestServer::fresh();
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    server
        .request(client.post(format!("{}/alerts", server.base_url)))
        .json(&message("from sse"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    let response = server
        .request(client.get(format!("{}/alerts/sse", server.base_url)))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    let line = BufReader::new(response)
        .lines()
        .map_while(Result::ok)
        .find(|line| line.starts_with("data:"))
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(line[5..].trim()).unwrap();
    assert_eq!(value["message"], "from sse");
}
