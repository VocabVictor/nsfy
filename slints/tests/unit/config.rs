use super::*;

#[test]
fn rejects_invalid_topic_keys() {
    assert!(topic_from_key("has spaces").is_none());
    assert!(topic_from_key("").is_none());
    assert!(topic_from_key("ops-prod_1").is_some());
}

#[test]
fn enforces_tls_for_remote_servers() {
    assert!(normalize_server_url("https://push.example.com").is_ok());
    assert!(normalize_server_url("http://localhost:8080").is_ok());
    assert!(normalize_server_url("http://127.0.0.1:8080").is_ok());
    assert!(normalize_server_url("http://192.168.1.20:8080").is_err());
    assert!(normalize_server_url("https://token@example.com").is_err());
}
