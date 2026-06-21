use crate::message::{Message, PublishRequest};
use crate::pubsub::PubSub;
use axum::extract::{ws, Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures_core::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

pub struct AppState {
    pub pubsub: PubSub,
    #[allow(dead_code)]
    pub auth_token: Option<String>,
}

/// GET / — health check + stats
pub async fn index(State(state): State<Arc<AppState>>) -> Json<crate::pubsub::PubSubStats> {
    Json(state.pubsub.stats())
}

/// POST /<topic> — publish a message
pub async fn publish(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    body: Result<Json<PublishRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    let req = body.map_err(|e| AppError::bad_request(e.to_string()))?;

    let msg: Message = req.0.into();
    let msg_json = serde_json::to_string(&msg).unwrap();

    debug!("publish to {}: {}", topic, msg_json);
    state.pubsub.publish(&topic, msg).await;

    Ok((StatusCode::OK, msg_json))
}

/// GET /<topic>/json?since=<id> — HTTP poll
pub async fn poll(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<Message>>, AppError> {
    let since_id = params.get("since").map(|s| s.as_str());
    let msgs = state.pubsub.get_messages(&topic, since_id).await;
    Ok(Json(msgs))
}

/// GET /<topic>/ws — WebSocket subscription
pub async fn ws_subscribe(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    debug!("ws upgrade requested for topic: {}", topic);
    ws.on_upgrade(move |socket| handle_ws(socket, state, topic))
}

async fn handle_ws(mut socket: ws::WebSocket, state: Arc<AppState>, topic: String) {
    info!("ws connected to topic: {}", topic);

    // Send cached messages first
    let cached = state.pubsub.get_messages_arc(&topic, None).await;
    for msg in &cached {
        let payload = serde_json::to_string(msg.as_ref()).unwrap();
        if socket.send(ws::Message::Text(payload.into())).await.is_err() {
            return;
        }
    }

    // Subscribe to live messages
    let (_, mut rx) = state.pubsub.subscribe(&topic);

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let payload = serde_json::to_string(msg.as_ref()).unwrap();
                        if socket.send(ws::Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("ws lagged by {} messages for topic: {}", n, topic);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(ws::Message::Text(text))) => {
                        // Client can also publish via WS
                        if let Ok(req) = serde_json::from_str::<PublishRequest>(&text) {
                            let msg: Message = req.into();
                            state.pubsub.publish(&topic, msg).await;
                        }
                    }
                    Some(Ok(ws::Message::Close(_))) | None => break,
                    Some(Ok(_)) => {} // ignore binary/ping/pong
                    Some(Err(e)) => {
                        warn!("ws error on topic {}: {}", topic, e);
                        break;
                    }
                }
            }
        }
    }

    let stats = state.pubsub.stats();
    info!(
        "ws disconnected from topic: {} ({} topics, {} total subscribers)",
        topic, stats.topics, stats.total_subscribers
    );
}

/// GET /<topic>/sse — Server-Sent Events subscription
pub async fn sse_subscribe(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("sse connected to topic: {}", topic);

    // Send cached messages first
    let cached = state.pubsub.get_messages(&topic, None).await;
    let cache_events: Vec<Result<Event, Infallible>> = cached
        .iter()
        .map(|msg| {
            let payload = serde_json::to_string(msg).unwrap();
            Ok(Event::default().data(payload))
        })
        .collect();

    let (_, rx) = state.pubsub.subscribe(&topic);
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        match result {
            Ok(msg) => {
                let payload = serde_json::to_string(msg.as_ref()).ok()?;
                Some(Ok(Event::default().data(payload)))
            }
            Err(_) => None,
        }
    });

    let combined = tokio_stream::iter(cache_events).chain(stream);
    Sse::new(combined).keep_alive(KeepAlive::default())
}

// --- Error handling ---

pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}

// Required for broadcast receiver
use tokio::sync::broadcast;
