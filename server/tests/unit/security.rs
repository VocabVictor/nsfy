use super::*;

#[test]
fn constant_time_comparison_accepts_only_equal_values() {
    assert!(constant_time_eq(b"secret", b"secret"));
    assert!(!constant_time_eq(b"secret", b"secre"));
    assert!(!constant_time_eq(b"secret", b"secrex"));
}

#[test]
fn rate_limiter_allows_burst_then_denies() {
    let limiter: RateLimiter<IpAddr> = RateLimiter::new(2, 0.0001);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    assert!(limiter.check(ip));
    assert!(limiter.check(ip));
    assert!(!limiter.check(ip));
}

#[test]
fn byte_limiter_accounts_for_payload_size() {
    let limiter: RateLimiter<IpAddr> = RateLimiter::new(1000, 0.0001);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    assert!(limiter.check_n(ip, 400.0));
    assert!(limiter.check_n(ip, 400.0));
    assert!(!limiter.check_n(ip, 400.0));
    assert!(!limiter.check_n("127.0.0.2".parse().unwrap(), 5000.0));
}

#[test]
fn topic_limiter_has_independent_buckets() {
    let limiter: RateLimiter<String> = RateLimiter::new(1, 0.0001);
    assert!(limiter.check("alerts".to_string()));
    assert!(!limiter.check("alerts".to_string()));
    assert!(limiter.check("backups".to_string()));
}

#[test]
fn bearer_auth_requires_the_authorization_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        "Bearer secret".parse().unwrap(),
    );
    assert_eq!(bearer_token(&headers), Some("secret"));
    assert_eq!(bearer_token(&HeaderMap::new()), None);
}

#[test]
fn connection_total_cap_is_atomic_under_contention() {
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
    let permits: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(permits.iter().filter(|permit| permit.is_some()).count(), 50);
    drop(permits);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let refill: Vec<_> = (0..max_total).map(|_| limiter.acquire(ip)).collect();
    assert!(refill.iter().all(|permit| permit.is_some()));
    assert!(limiter.acquire(ip).is_none());
}

#[test]
fn connection_limiter_enforces_per_ip_and_total_caps() {
    let limiter = ConnLimiter::new(2, 3);
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let other: IpAddr = "127.0.0.2".parse().unwrap();
    let first = limiter.acquire(ip);
    let second = limiter.acquire(ip);
    assert!(first.is_some() && second.is_some());
    assert!(limiter.acquire(ip).is_none());
    let third = limiter.acquire(other);
    assert!(third.is_some());
    assert!(limiter.acquire(other).is_none());
    drop(first);
    assert!(limiter.acquire(ip).is_some());
}

#[test]
fn forwarded_headers_are_ignored_unless_trusted() {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
    let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();
    assert_eq!(client_ip(&headers, peer, false), peer.ip());
    assert_eq!(
        client_ip(&headers, peer, true),
        "9.9.9.9".parse::<IpAddr>().unwrap()
    );
}
