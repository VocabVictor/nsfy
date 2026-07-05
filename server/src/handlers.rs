use crate::message::{Message, PublishRequest};
use crate::pubsub::PubSub;
use crate::security::{client_ip, ConnLimiter, ConnPermit, RateLimiter};
use axum::extract::{ws, ConnectInfo, Path, Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures_core::stream::Stream;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

pub struct AppState {
    pub pubsub: PubSub,
    pub auth_token: Option<String>,
    pub rate_limiter: RateLimiter<IpAddr>,
    pub topic_rate_limiter: RateLimiter<String>,
    pub conn_limiter: ConnLimiter,
    pub max_msg_size: usize,
    pub trust_proxy: bool,
}

const MAX_TOPIC_LEN: usize = 128;
const MAX_TITLE_LEN: usize = 512;
const MAX_TAGS: usize = 32;
const MAX_TAG_LEN: usize = 64;

/// Topic names land in log lines and DashMap keys, never in a filesystem
/// path, so this isn't about traversal — it's about keeping out control
/// characters (log injection) and unbounded-length keys.
fn is_valid_topic(topic: &str) -> bool {
    !topic.is_empty()
        && topic.len() <= MAX_TOPIC_LEN
        && topic
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Field-level bounds so a single message can't bloat a topic's cache or
/// its broadcast payload, even though the outer HTTP body size is already
/// capped by `DefaultBodyLimit`.
fn validate_publish(req: &PublishRequest, max_msg_size: usize) -> Result<(), &'static str> {
    if req.message.len() > max_msg_size {
        return Err("message too large");
    }
    if req.title.len() > MAX_TITLE_LEN {
        return Err("title too large");
    }
    if req.tags.len() > MAX_TAGS {
        return Err("too many tags");
    }
    if req.tags.iter().any(|t| t.len() > MAX_TAG_LEN) {
        return Err("tag too large");
    }
    Ok(())
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
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    let req = body.map_err(|e| AppError::bad_request(e.to_string()))?;
    validate_publish(&req, state.max_msg_size).map_err(AppError::bad_request)?;

    // Aggregate per-topic budget, on top of the per-IP one already applied
    // by the global middleware — stops a distributed flood (many IPs, one
    // topic) that per-IP limiting alone can't catch.
    if !state.topic_rate_limiter.check(topic.clone()) {
        return Err(AppError::too_many_requests(
            "topic publish rate limit exceeded",
        ));
    }

    let msg: Message = req.0.into();
    let msg_json = serde_json::to_string(&msg).unwrap();

    if !state.pubsub.publish(&topic, msg).await {
        return Err(AppError::unavailable("topic table full, try again later"));
    }
    debug!("publish to {}: {}", topic, msg_json);

    Ok((StatusCode::OK, msg_json))
}

/// GET /<topic>/json?since=<id> — HTTP poll
pub async fn poll(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<Message>>, AppError> {
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    let since_id = params.get("since").map(|s| s.as_str());
    match state.pubsub.get_messages(&topic, since_id).await {
        Some(msgs) => Ok(Json(msgs)),
        None => Err(AppError::unavailable("topic table full, try again later")),
    }
}

/// GET /<topic>/ws — WebSocket subscription
pub async fn ws_subscribe(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    let ip = client_ip(&headers, addr, state.trust_proxy);
    let Some(permit) = state.conn_limiter.acquire(ip) else {
        return Err(AppError::too_many_requests(
            "too many concurrent connections",
        ));
    };
    debug!("ws upgrade requested for topic: {}", topic);
    // Frame/message size bounded to the configured message limit (plus JSON
    // structure overhead) so a client can't force a large allocation before
    // any application-level validation runs.
    let ws_limit = state.max_msg_size.saturating_add(4096);
    Ok(ws
        .max_message_size(ws_limit)
        .max_frame_size(ws_limit)
        .on_upgrade(move |socket| handle_ws(socket, state, topic, ip, permit)))
}

async fn handle_ws(
    mut socket: ws::WebSocket,
    state: Arc<AppState>,
    topic: String,
    ip: IpAddr,
    _permit: ConnPermit,
) {
    info!("ws connected to topic: {}", topic);

    // Send cached messages first
    let Some(cached) = state.pubsub.get_messages_arc(&topic, None).await else {
        let _ = socket.send(ws::Message::Close(None)).await;
        return;
    };
    for msg in &cached {
        let payload = serde_json::to_string(msg.as_ref()).unwrap();
        if socket
            .send(ws::Message::Text(payload.into()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Subscribe to live messages
    let Some((_, mut rx)) = state.pubsub.subscribe(&topic) else {
        let _ = socket.send(ws::Message::Close(None)).await;
        return;
    };

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
                        // Client can also publish via WS — same rate limits and
                        // field checks as the HTTP path, since this bypasses it.
                        if !state.rate_limiter.check(ip) || !state.topic_rate_limiter.check(topic.clone()) {
                            continue;
                        }
                        if let Ok(req) = serde_json::from_str::<PublishRequest>(&text) {
                            if validate_publish(&req, state.max_msg_size).is_ok() {
                                let msg: Message = req.into();
                                state.pubsub.publish(&topic, msg).await;
                            }
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    let ip = client_ip(&headers, addr, state.trust_proxy);
    let Some(permit) = state.conn_limiter.acquire(ip) else {
        return Err(AppError::too_many_requests(
            "too many concurrent connections",
        ));
    };
    info!("sse connected to topic: {}", topic);

    // Send cached messages first
    let Some(cached) = state.pubsub.get_messages(&topic, None).await else {
        return Err(AppError::unavailable("topic table full, try again later"));
    };
    let cache_events: Vec<Result<Event, Infallible>> = cached
        .iter()
        .map(|msg| {
            let payload = serde_json::to_string(msg).unwrap();
            Ok(Event::default().data(payload))
        })
        .collect();

    let Some((_, rx)) = state.pubsub.subscribe(&topic) else {
        return Err(AppError::unavailable("topic table full, try again later"));
    };
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(msg) => {
            let payload = serde_json::to_string(msg.as_ref()).ok()?;
            Some(Ok(Event::default().data(payload)))
        }
        Err(_) => None,
    });

    let combined = WithPermit {
        inner: tokio_stream::iter(cache_events).chain(stream),
        _permit: permit,
    };
    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}

/// Wraps a stream together with a `ConnPermit` so the connection-count slot
/// is released exactly when the SSE stream is dropped (client disconnects,
/// or the response future is cancelled) — not before.
struct WithPermit<S> {
    inner: S,
    _permit: ConnPermit,
}

impl<S: Stream + Unpin> Stream for WithPermit<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
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

    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: msg.into(),
        }
    }

    pub fn too_many_requests(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
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
