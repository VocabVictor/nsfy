mod common;

use common::{message, TestServer, TOKEN};
use reqwest::blocking::Client;
use serde_json::Value;

#[test]
fn publish_poll_and_since_round_trip_all_fields() {
    let server = TestServer::fresh();
    let client = Client::new();
    let first: Value = server
        .request(client.post(format!("{}/alerts", server.base_url)))
        .json(&message("first"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .unwrap();
    let second: Value = server
        .request(client.post(format!("{}/alerts", server.base_url)))
        .json(&message("second"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .unwrap();
    let all: Vec<Value> = server
        .request(client.get(format!("{}/alerts/json", server.base_url)))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0]["category"], serde_json::json!(["qa", "protocol"]));
    let since: Vec<Value> = server
        .request(client.get(format!(
            "{}/alerts/json?since={}",
            server.base_url,
            first["id"].as_str().unwrap()
        )))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(since, vec![second]);
}

#[test]
fn bearer_header_is_required_and_query_tokens_are_rejected() {
    let server = TestServer::fresh();
    let client = Client::new();
    let missing = client
        .get(format!("{}/alerts/json", server.base_url))
        .send()
        .unwrap();
    assert_eq!(missing.status(), 401);
    let query = client
        .get(format!("{}/alerts/json?auth={TOKEN}", server.base_url))
        .send()
        .unwrap();
    assert_eq!(query.status(), 401);
    let wrong = client
        .get(format!("{}/alerts/json", server.base_url))
        .bearer_auth("wrong")
        .send()
        .unwrap();
    assert_eq!(wrong.status(), 401);
}

#[test]
fn invalid_topics_and_oversized_fields_are_rejected() {
    let server = TestServer::fresh();
    let client = Client::new();
    let invalid = server
        .request(client.post(format!("{}/bad%20topic", server.base_url)))
        .json(&message("body"))
        .send()
        .unwrap();
    assert_eq!(invalid.status(), 400);
    let deep = serde_json::json!({
        "message": "body",
        "category": ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
    });
    let response = server
        .request(client.post(format!("{}/alerts", server.base_url)))
        .json(&deep)
        .send()
        .unwrap();
    assert_eq!(response.status(), 400);
}
