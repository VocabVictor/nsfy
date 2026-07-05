mod config;
mod handlers;
mod message;
mod pubsub;
mod security;

use axum::extract::DefaultBodyLimit;
use axum::{middleware, routing::get, routing::post, Router};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use handlers::AppState;
use pubsub::PubSub;
use security::{ConnLimiter, RateLimiter};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nsfyd=info,tower_http=info".into()),
        )
        .with_target(false)
        .init();

    let cfg = Config::parse();

    // Burst = one-sixth of the per-minute budget (min 10); refills to the
    // full per-minute rate spread evenly across each second.
    let burst = (cfg.rate_limit_per_min / 6).max(10);
    let refill_per_sec = cfg.rate_limit_per_min as f64 / 60.0;

    let topic_burst = (cfg.topic_rate_limit_per_min / 6).max(10);
    let topic_refill_per_sec = cfg.topic_rate_limit_per_min as f64 / 60.0;

    let state = Arc::new(AppState {
        pubsub: PubSub::new(cfg.cache_size, cfg.max_topics),
        auth_token: cfg.auth_token.clone(),
        rate_limiter: RateLimiter::new(burst, refill_per_sec),
        topic_rate_limiter: RateLimiter::new(topic_burst, topic_refill_per_sec),
        conn_limiter: ConnLimiter::new(cfg.max_conns_per_ip, cfg.max_conns_total),
        max_msg_size: cfg.max_msg_size,
        trust_proxy: cfg.trust_proxy,
    });

    if cfg.stats_interval > 0 {
        let state_clone = Arc::clone(&state);
        let interval = cfg.stats_interval;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(interval)).await;
                let stats = state_clone.pubsub.stats();
                info!(
                    "stats: {} topics, {} total subscribers | topics: {:?}",
                    stats.topics, stats.total_subscribers, stats.topic_names
                );
            }
        });
    }

    // Sweep idle rate-limit buckets and empty connection counters so the
    // tracking tables don't grow forever under a rotating-IP flood.
    {
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;
                state_clone.rate_limiter.sweep(Duration::from_secs(600));
                state_clone
                    .topic_rate_limiter
                    .sweep(Duration::from_secs(600));
                state_clone.conn_limiter.sweep();
            }
        });
    }

    let body_limit = cfg.max_msg_size.saturating_add(4096);

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/{topic}", post(handlers::publish))
        .route("/{topic}/ws", get(handlers::ws_subscribe))
        .route("/{topic}/sse", get(handlers::sse_subscribe))
        .route("/{topic}/json", get(handlers::poll))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            security::guard,
        ))
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        // Turns a panic in any single request into a 500 response instead of
        // taking down every other client's connection with the process.
        .layer(CatchPanicLayer::new())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.listen).await.unwrap();
    info!("nsfyd listening on {}", cfg.listen);
    info!(
        "cache_size={}, max_msg_size={}, max_topics={}, rate_limit_per_min={}, \
         topic_rate_limit_per_min={}, max_conns_per_ip={}, max_conns_total={}, \
         trust_proxy={}, auth={}",
        cfg.cache_size,
        cfg.max_msg_size,
        cfg.max_topics,
        cfg.rate_limit_per_min,
        cfg.topic_rate_limit_per_min,
        cfg.max_conns_per_ip,
        cfg.max_conns_total,
        cfg.trust_proxy,
        cfg.auth_token.is_some()
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
