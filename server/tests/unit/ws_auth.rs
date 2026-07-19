use super::*;

#[test]
fn accepts_native_clients_and_known_tauri_origins() {
    assert!(origin_allowed(&HeaderMap::new()));
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::ORIGIN,
        "tauri://localhost".parse().unwrap(),
    );
    assert!(origin_allowed(&headers));
    headers.insert(
        axum::http::header::ORIGIN,
        "https://malicious.example".parse().unwrap(),
    );
    assert!(!origin_allowed(&headers));
}
