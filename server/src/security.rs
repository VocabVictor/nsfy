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

/// Global middleware: rate-limit every request by client IP, then — if an
/// auth token is configured — require it on every route (including `/`, so
/// topic names can't be enumerated by an unauthenticated caller).
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

    if let Some(expected) = &state.auth_token {
        let provided = bearer_token(&req).or_else(|| query_param(req.uri().query(), "auth"));
        let ok = match provided {
            Some(p) => constant_time_eq(p.as_bytes(), expected.as_bytes()),
            None => false,
        };
        if !ok {
            return (StatusCode::UNAUTHORIZED, "missing or invalid auth token").into_response();
        }
    }

    next.run(req).await
}

fn bearer_token(req: &Request) -> Option<String> {
    let value = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value.strip_prefix("Bearer ").map(|s| s.to_string())
}

fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    let query = query?;
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            return it.next().map(|v| v.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_equal_slices() {
        assert!(constant_time_eq(b"secret", b"secret"));
    }

    #[test]
    fn constant_time_eq_rejects_different_length_or_content() {
        assert!(!constant_time_eq(b"secret", b"secre"));
        assert!(!constant_time_eq(b"secret", b"secrex"));
    }

    #[test]
    fn rate_limiter_allows_burst_then_denies() {
        let rl: RateLimiter<IpAddr> = RateLimiter::new(2, 0.0001);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(rl.check(ip));
        assert!(rl.check(ip));
        assert!(!rl.check(ip));
    }

    #[test]
    fn check_n_meters_bytes_not_just_request_count() {
        // A byte-budget bucket of 1000, refilling negligibly slowly: two
        // 400-byte messages fit, a third 400-byte one doesn't even though
        // only 2 "requests" have happened, and a single request larger than
        // the whole budget is rejected outright rather than partially spent.
        let rl: RateLimiter<IpAddr> = RateLimiter::new(1000, 0.0001);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(rl.check_n(ip, 400.0));
        assert!(rl.check_n(ip, 400.0));
        assert!(!rl.check_n(ip, 400.0));

        let ip2: IpAddr = "127.0.0.2".parse().unwrap();
        assert!(!rl.check_n(ip2, 5000.0));
    }

    #[test]
    fn topic_rate_limiter_keys_by_string() {
        let rl: RateLimiter<String> = RateLimiter::new(1, 0.0001);
        assert!(rl.check("alerts".to_string()));
        assert!(!rl.check("alerts".to_string()));
        assert!(rl.check("backups".to_string()));
    }

    #[test]
    fn query_param_extracts_value() {
        assert_eq!(
            query_param(Some("a=1&auth=tok123"), "auth"),
            Some("tok123".to_string())
        );
        assert_eq!(query_param(Some("a=1"), "auth"), None);
    }

    /// Regression test for the increment-then-check TOCTOU fix: hammer
    /// `acquire` from many threads at once — a "peek total, then increment"
    /// implementation would let concurrent callers overshoot `max_total`
    /// right around this test's contention window.
    #[test]
    fn conn_limiter_never_exceeds_total_cap_under_concurrency() {
        use std::sync::Barrier;
        use std::thread;

        let max_total = 50;
        let limiter = Arc::new(ConnLimiter::new(u32::MAX, max_total));
        let threads = 200;
        let barrier = Arc::new(Barrier::new(threads));

        let handles: Vec<_> = (0..threads)
            .map(|i| {
                let limiter = Arc::clone(&limiter);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let ip: IpAddr = format!("10.0.{}.{}", i / 256, i % 256).parse().unwrap();
                    barrier.wait();
                    limiter.acquire(ip)
                })
            })
            .collect();

        let permits: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let granted = permits.iter().filter(|p| p.is_some()).count();
        assert_eq!(
            granted, max_total as usize,
            "exactly max_total permits should be granted, no more"
        );

        // Freeing everything must return the counters to exactly zero, not
        // leave them skewed from the rollback bookkeeping — so the full
        // budget is acquirable again, held all at once.
        drop(permits);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let refill: Vec<_> = (0..max_total).map(|_| limiter.acquire(ip)).collect();
        assert!(refill.iter().all(|p| p.is_some()));
        assert!(
            limiter.acquire(ip).is_none(),
            "budget should be exhausted again, not over-refilled"
        );
    }

    #[test]
    fn conn_limiter_enforces_per_ip_and_total_caps() {
        let cl = ConnLimiter::new(2, 3);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let other: IpAddr = "127.0.0.2".parse().unwrap();
        let p1 = cl.acquire(ip);
        let p2 = cl.acquire(ip);
        assert!(p1.is_some() && p2.is_some());
        assert!(
            cl.acquire(ip).is_none(),
            "per-IP cap should reject a 3rd connection"
        );
        // Bind the permit — an unbound temporary would drop (and free its
        // slot) at the end of its statement, defeating this check.
        let p3 = cl.acquire(other);
        assert!(p3.is_some(), "different IP has its own budget");
        assert!(
            cl.acquire(other).is_none(),
            "total cap of 3 should now be hit"
        );
        drop(p1);
        assert!(
            cl.acquire(ip).is_some(),
            "freeing a permit should free up budget"
        );
    }

    #[test]
    fn client_ip_ignores_forwarded_headers_unless_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
        let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        assert_eq!(client_ip(&headers, peer, false), peer.ip());
        assert_eq!(
            client_ip(&headers, peer, true),
            "9.9.9.9".parse::<IpAddr>().unwrap()
        );
    }
}
