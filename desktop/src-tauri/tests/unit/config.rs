use super::*;

#[test]
fn permits_https_and_loopback_http() {
    assert!(normalize_url("https://push.example.com").is_ok());
    assert!(normalize_url("http://localhost:8080").is_ok());
    assert!(normalize_url("http://127.0.0.1:8080").is_ok());
    assert!(normalize_url("http://[::1]:8080").is_ok());
}

#[test]
fn rejects_remote_cleartext_and_url_credentials() {
    assert_eq!(
        normalize_url("http://192.168.1.20:8080").unwrap_err(),
        "remote servers must use https://"
    );
    assert!(normalize_url("https://token@example.com").is_err());
}

#[test]
fn legacy_config_defaults_to_resident_notifications() {
    let config: StoredConfig = serde_json::from_str(r#"{"servers":[],"topics":[]}"#).unwrap();
    assert_eq!(config.window_behavior, "resident");
    assert!(!config.do_not_disturb);
}
