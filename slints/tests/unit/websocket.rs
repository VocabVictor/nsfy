use super::*;

#[test]
fn builds_websocket_urls_without_credentials() {
    assert_eq!(
        websocket_url("https://push.example/", "alerts"),
        "wss://push.example/alerts/ws"
    );
}
