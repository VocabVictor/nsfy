use crate::access::Permission;
use crate::handlers::{is_valid_topic, AppError, AppState};
use crate::security::{client_ip, ConnPermit};
use axum::extract::{ws, ConnectInfo, Path, State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

pub async fn subscribe(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    if !crate::ws_auth::origin_allowed(&headers) {
        return Err(AppError::forbidden("WebSocket origin is not allowed"));
    }
    let ip = client_ip(&headers, addr, state.trust_proxy);
    let Some(permit) = state.conn_limiter.acquire(ip) else {
        return Err(AppError::too_many_requests(
            "too many concurrent connections",
        ));
    };
    debug!("ws upgrade requested for topic: {}", topic);
    let ws_limit = state.max_msg_size.saturating_add(4096);
    Ok(ws
        .read_buffer_size(4096)
        .write_buffer_size(4096)
        .max_write_buffer_size(ws_limit.saturating_mul(2))
        .max_message_size(ws_limit)
        .max_frame_size(ws_limit)
        .on_upgrade(move |socket| handle(socket, state, topic, permit, headers)))
}

async fn handle(
    mut socket: ws::WebSocket,
    state: Arc<AppState>,
    topic: String,
    _permit: ConnPermit,
    headers: HeaderMap,
) {
    if !crate::ws_auth::authenticate(&mut socket, &headers, |token| {
        state.access.allows_topic(&topic, Permission::Read, token)
    })
    .await
    {
        warn!("ws authentication failed for topic: {}", topic);
        return;
    }
    info!("ws connected to topic: {}", topic);
    let Some(cached) = state.pubsub.get_messages_arc(&topic, None).await else {
        let _ = socket.send(ws::Message::Close(None)).await;
        return;
    };
    for message in &cached {
        let payload = serde_json::to_string(message.as_ref()).unwrap();
        if socket
            .send(ws::Message::Text(payload.into()))
            .await
            .is_err()
        {
            return;
        }
    }
    let Some((_, mut receiver)) = state.pubsub.subscribe(&topic) else {
        let _ = socket.send(ws::Message::Close(None)).await;
        return;
    };

    loop {
        tokio::select! {
            result = receiver.recv() => match result {
                Ok(message) => {
                    let payload = serde_json::to_string(message.as_ref()).unwrap();
                    if socket.send(ws::Message::Text(payload.into())).await.is_err() { break; }
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!("ws lagged by {} messages for topic: {}", count, topic);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            message = socket.recv() => match message {
                Some(Ok(ws::Message::Close(_))) | None => break,
                Some(Err(error)) => {
                    warn!("ws error on topic {}: {}", topic, error);
                    break;
                }
                _ => {}
            }
        }
    }
    let stats = state.pubsub.stats();
    info!(
        "ws disconnected from topic: {} ({} topics, {} total subscribers)",
        topic, stats.topics, stats.total_subscribers
    );
}
