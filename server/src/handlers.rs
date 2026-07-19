use crate::access::AccessControl;
use crate::message::{Message, PublishRequest};
use crate::pubsub::PubSub;
use crate::security::{client_ip, ConnLimiter, ConnPermit, RateLimiter};
use crate::store::Persistence;
use axum::extract::{ConnectInfo, Path, Query, State};
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
use tracing::{debug, info};

pub struct AppState {
    pub pubsub: PubSub,
    pub access: AccessControl,
    pub rate_limiter: RateLimiter<IpAddr>,
    pub topic_rate_limiter: RateLimiter<String>,
    pub bandwidth_limiter: RateLimiter<IpAddr>,
    pub topic_creation_limiter: RateLimiter<IpAddr>,
    pub conn_limiter: ConnLimiter,
    pub max_msg_size: usize,
    pub trust_proxy: bool,
    pub persistence: Persistence,
    pub db_keep_per_topic: usize,
}

const MAX_TOPIC_LEN: usize = 128;
const MAX_TITLE_LEN: usize = 512;
const MAX_TAGS: usize = 32;
const MAX_TAG_LEN: usize = 64;
const MAX_CATEGORY_DEPTH: usize = 8;
const MAX_CATEGORY_SEGMENT_LEN: usize = 64;

/// Topic names land in log lines and DashMap keys, never in a filesystem
/// path, so this isn't about traversal — it's about keeping out control
/// characters (log injection) and unbounded-length keys.
pub(crate) fn is_valid_topic(topic: &str) -> bool {
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
    if req.category.len() > MAX_CATEGORY_DEPTH {
        return Err("category is too deep");
    }
    if req.category.iter().any(|segment| {
        segment.trim().is_empty()
            || segment.len() > MAX_CATEGORY_SEGMENT_LEN
            || segment.chars().any(char::is_control)
    }) {
        return Err("invalid category segment");
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<PublishRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    let req = body.map_err(|e| AppError::bad_request(e.to_string()))?;
    validate_publish(&req, state.max_msg_size).map_err(AppError::bad_request)?;

    let ip = client_ip(&headers, addr, state.trust_proxy);

    // A brand-new topic draws from a separate, tighter per-IP budget than
    // publishes to topics that already exist — caps how fast one IP can
    // spend the shared `max_topics` headroom, on top of the hard ceiling.
    if !state.pubsub.topic_exists(&topic) && !state.topic_creation_limiter.check(ip) {
        return Err(AppError::too_many_requests(
            "topic creation rate limit exceeded",
        ));
    }

    // Bytes, not just requests: a handful of maximum-size messages would
    // otherwise slip under a request-count-only limit.
    if !state
        .bandwidth_limiter
        .check_n(ip, req.message.len() as f64)
    {
        return Err(AppError::too_many_requests("bandwidth limit exceeded"));
    }

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
    state
        .persistence
        .record(&topic, &msg, state.db_keep_per_topic)
        .await
        .map_err(AppError::unavailable)?;

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

pub(crate) struct AppError {
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

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
