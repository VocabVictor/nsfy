use crate::security;
use axum::extract::ws::{Message, WebSocket};
use axum::http::HeaderMap;
use serde::Deserialize;
use std::time::Duration;

const ALLOWED_ORIGINS: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "http://localhost:1420",
];

#[derive(Deserialize)]
struct AuthFrame {
    #[serde(rename = "type")]
    kind: String,
    token: String,
}

pub fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(axum::http::header::ORIGIN) else {
        return true;
    };
    origin
        .to_str()
        .ok()
        .is_some_and(|origin| ALLOWED_ORIGINS.contains(&origin))
}

pub async fn authenticate<F>(socket: &mut WebSocket, headers: &HeaderMap, allowed: F) -> bool
where
    F: Fn(Option<&str>) -> bool,
{
    let header_token = security::bearer_token(headers);
    if allowed(header_token) {
        return true;
    }
    if header_token.is_some() {
        let _ = socket.send(Message::Close(None)).await;
        return false;
    }

    let frame = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let token = match frame {
        Ok(Some(Ok(Message::Text(text)))) => serde_json::from_str::<AuthFrame>(&text)
            .ok()
            .filter(|frame| frame.kind == "auth")
            .map(|frame| frame.token),
        _ => None,
    };
    let valid = allowed(token.as_deref());
    if !valid {
        let _ = socket.send(Message::Close(None)).await;
    }
    valid
}

#[cfg(test)]
#[path = "../tests/unit/ws_auth.rs"]
mod tests;
