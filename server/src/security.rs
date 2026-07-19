use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use std::hash::Hash;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::access::Permission;
use crate::handlers::AppState;

/// Compare two byte strings in constant time, so a wrong token takes the same
/// time as a right one regardless of how many leading bytes match.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

/// Hard cap on distinct keys tracked at once, so a distributed flood (many
/// IPs) or a topic-name-spraying attacker can't grow this map without
/// bound. Past the cap, unseen keys are denied instead of allocated a
/// bucket.
const MAX_TRACKED_KEYS: usize = 50_000;

/// Generic token bucket, one per key (client IP for the global limiter,
/// topic name for the per-topic limiter). One DashMap lookup plus a
/// short-held std Mutex per check — no async wait, no allocation on the
/// hot path once warm.
pub struct RateLimiter<K: Eq + Hash + Clone> {
    buckets: DashMap<K, Mutex<Bucket>>,
    capacity: f64,
    refill_per_sec: f64,
}

impl<K: Eq + Hash + Clone> RateLimiter<K> {
    /// `capacity` is `u64` rather than `u32` so this can meter either
    /// request counts or byte counts (a bandwidth budget can easily exceed
    /// 4 billion in one burst window).
    pub fn new(capacity: u64, refill_per_sec: f64) -> Self {
        Self {
            buckets: DashMap::new(),
            capacity: capacity.max(1) as f64,
            refill_per_sec: refill_per_sec.max(0.001),
        }
    }

    /// Returns true if the request is allowed, false if the key is over budget.
    pub fn check(&self, key: K) -> bool {
        self.check_n(key, 1.0)
    }

    /// Same as `check`, but draws `cost` tokens instead of 1 — lets one
    /// bucket meter something other than request count (e.g. bytes
    /// published), reusing the same refill/burst machinery.
    pub fn check_n(&self, key: K, cost: f64) -> bool {
        if self.buckets.get(&key).is_none() && self.buckets.len() >= MAX_TRACKED_KEYS {
            // Fail closed for new keys once the tracking table is full,
            // rather than let memory grow without bound.
            return false;
        }

        let entry = self.buckets.entry(key).or_insert_with(|| {
            Mutex::new(Bucket {
                tokens: self.capacity,
                last: Instant::now(),
            })
        });
        // Recover from a poisoned lock rather than panicking forever on
        // every future request from this key — a panic elsewhere while
        // holding this lock must not permanently break its rate limiting.
        let mut bucket = entry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.last = now;
        bucket.tokens = (bucket.tokens + elapsed * self.refill_per_sec).min(self.capacity);

        if bucket.tokens >= cost {
            bucket.tokens -= cost;
            true
        } else {
            false
        }
    }

    /// Drop buckets that haven't been touched in a while, so the table
    /// doesn't grow forever under a rotating-key flood.
    pub fn sweep(&self, idle: Duration) {
        let now = Instant::now();
        self.buckets.retain(|_, bucket| match bucket.try_lock() {
            Ok(b) => now.duration_since(b.last) < idle,
            Err(_) => true,
        });
    }
}

/// Caps concurrent long-lived connections (WS/SSE) per IP and server-wide,
/// so opening many sockets — which the per-request rate limiter only
/// charges once each — can't exhaust memory or file descriptors.
pub struct ConnLimiter {
    per_ip: DashMap<IpAddr, Arc<AtomicU32>>,
    total: Arc<AtomicU32>,
    max_per_ip: u32,
    max_total: u32,
}

/// RAII permit — decrements the counters when the connection closes,
/// however it closes (clean disconnect, error, or the task getting dropped).
pub struct ConnPermit {
    per_ip: Arc<AtomicU32>,
    total: Arc<AtomicU32>,
}

impl Drop for ConnPermit {
    fn drop(&mut self) {
        self.per_ip.fetch_sub(1, Ordering::Relaxed);
        self.total.fetch_sub(1, Ordering::Relaxed);
    }
}

impl ConnLimiter {
    pub fn new(max_per_ip: u32, max_total: u32) -> Self {
        Self {
            per_ip: DashMap::new(),
            total: Arc::new(AtomicU32::new(0)),
            max_per_ip,
            max_total,
        }
    }

    pub fn acquire(&self, ip: IpAddr) -> Option<ConnPermit> {
        let counter = self
            .per_ip
            .entry(ip)
            .or_insert_with(|| Arc::new(AtomicU32::new(0)))
            .clone();

        // Increment-then-check-then-rollback for both counters, never
        // check-then-increment: a "peek, then act" gap lets concurrent
        // callers all pass the check before any of them lands their
        // increment, so the cap gets overshot under exactly the connection
        // burst this limiter exists to stop.
        if counter.fetch_add(1, Ordering::Relaxed) >= self.max_per_ip {
            counter.fetch_sub(1, Ordering::Relaxed);
            return None;
        }
        if self.total.fetch_add(1, Ordering::Relaxed) >= self.max_total {
            self.total.fetch_sub(1, Ordering::Relaxed);
            counter.fetch_sub(1, Ordering::Relaxed);
            return None;
        }
        Some(ConnPermit {
            per_ip: counter,
            total: Arc::clone(&self.total),
        })
    }

    /// Drop per-IP counters that have gone back to zero, so the map doesn't
    /// grow forever as distinct clients come and go.
    pub fn sweep(&self) {
        self.per_ip.retain(|_, c| c.load(Ordering::Relaxed) > 0);
    }
}

/// Resolve the client IP used for rate limiting. By default this is the TCP
/// peer address. When `trust_proxy` is set, `X-Forwarded-For` (first hop)
/// or `X-Real-IP` is preferred instead — only safe when nsfyd is reachable
/// exclusively through a reverse proxy that sets/overwrites these headers,
/// otherwise a direct client can forge them to dodge its own rate limit.
pub fn client_ip(headers: &HeaderMap, peer: SocketAddr, trust_proxy: bool) -> IpAddr {
    if trust_proxy {
        if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = xff.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
        if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            if let Ok(ip) = real_ip.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }
    peer.ip()
}

/// WebSocket upgrades authenticate inside the socket before any message is
/// sent because browser WebSocket APIs cannot set Authorization headers.
/// Every other request authenticates here.
pub async fn guard(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    let ip = client_ip(req.headers(), addr, state.trust_proxy);

    if !state.rate_limiter.check(ip) {
        return (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
    }

    let websocket_upgrade = req.uri().path().ends_with("/ws")
        && req
            .headers()
            .get(axum::http::header::UPGRADE)
            .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"websocket"));
    if !websocket_upgrade {
        let token = bearer_token(req.headers());
        let path = req.uri().path().trim_matches('/');
        let allowed = if path.is_empty() {
            state.access.allows_global(token)
        } else {
            let topic = path.split('/').next().unwrap_or_default();
            let permission = if req.method() == Method::POST {
                Permission::Write
            } else {
                Permission::Read
            };
            state.access.allows_topic(topic, permission, token)
        };
        if !allowed {
            return (StatusCode::UNAUTHORIZED, "missing or invalid auth token").into_response();
        }
    }

    next.run(req).await
}

pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value.strip_prefix("Bearer ")
}

#[cfg(test)]
#[path = "../tests/unit/security.rs"]
mod tests;
