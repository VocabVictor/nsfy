use crate::access::Permission;
use crate::handlers::{is_valid_topic, AppError, AppState};
use crate::security::{client_ip, ConnPermit};
use crate::state_store::{validate_request, StateEvent, StateRequest};
use axum::extract::{ws, ConnectInfo, Path, State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub struct StateHub {
    channels: DashMap<String, broadcast::Sender<Arc<StateEvent>>>,
}

impl StateHub {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
        }
    }

    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<Arc<StateEvent>> {
        self.sender(topic).subscribe()
    }

    pub fn publish(&self, topic: &str, event: StateEvent) {
        if let Some(sender) = self.channels.get(topic) {
            let _ = sender.send(Arc::new(event));
        }
    }

    fn sender(&self, topic: &str) -> broadcast::Sender<Arc<StateEvent>> {
        self.channels
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    }
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(topic): Path<String>,
    Json(request): Json<StateRequest>,
) -> Result<Json<StateEvent>, AppError> {
    if !is_valid_topic(&topic) {
        return Err(AppError::bad_request("invalid topic name"));
    }
    if !state.pubsub.topic_exists(&topic) {
        return Err(AppError::bad_request("topic does not exist"));
    }
    validate_request(&request).map_err(AppError::bad_request)?;
    let store = Arc::clone(&state.state_store);
    let topic_for_store = topic.clone();
    let keep = state.state_keep_per_topic;
    let updates =
        tokio::task::spawn_blocking(move || store.record(&topic_for_store, &request.updates, keep))
            .await
            .map_err(|error| AppError::unavailable(error.to_string()))?
            .map_err(|error| AppError::unavailable(error.to_string()))?;
    let event = StateEvent {
        kind: "state",
        updates,
    };
    state.state_hub.publish(&topic, event.clone());
    Ok(Json(event))
}

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
    Ok(ws.on_upgrade(move |socket| handle(socket, state, topic, permit, headers)))
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
        warn!("state ws authentication failed for topic: {}", topic);
        return;
    }
    let mut receiver = state.state_hub.subscribe(&topic);
    let store = Arc::clone(&state.state_store);
    let snapshot_topic = topic.clone();
    let keep = state.state_keep_per_topic;
    let snapshot = tokio::task::spawn_blocking(move || store.load(&snapshot_topic, keep)).await;
    let Ok(Ok(updates)) = snapshot else {
        let _ = socket.send(ws::Message::Close(None)).await;
        return;
    };
    let initial = StateEvent {
        kind: "snapshot",
        updates,
    };
    if send_event(&mut socket, &initial).await.is_err() {
        return;
    }
    info!("state ws connected to topic: {}", topic);
    loop {
        tokio::select! {
            event = receiver.recv() => match event {
                Ok(event) => if send_event(&mut socket, &event).await.is_err() { break; },
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            frame = socket.recv() => match frame {
                Some(Ok(ws::Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            }
        }
    }
}

async fn send_event(socket: &mut ws::WebSocket, event: &StateEvent) -> Result<(), ()> {
    let payload = serde_json::to_string(event).map_err(|_| ())?;
    socket
        .send(ws::Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}
