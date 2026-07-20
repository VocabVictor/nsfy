mod access;
mod config;
mod handlers;
mod message;
mod pubsub;
mod security;
mod state_store;
mod state_sync;
mod store;
mod ws;
mod ws_auth;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method};
use axum::{middleware, routing::get, routing::post, Router};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use handlers::AppState;
use pubsub::PubSub;
use security::{ConnLimiter, RateLimiter};
use store::Persistence;

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
    let listen_addr: SocketAddr = cfg.listen.parse().unwrap_or_else(|error| {
        tracing::error!("invalid --listen address {}: {}", cfg.listen, error);
        std::process::exit(2);
    });
    let tls_enabled = match (&cfg.tls_cert, &cfg.tls_key) {
        (Some(_), Some(_)) => true,
        (None, None) => false,
        _ => {
            tracing::error!("--tls-cert and --tls-key must be configured together");
            std::process::exit(2);
        }
    };
    if !listen_addr.ip().is_loopback() && !tls_enabled {
        tracing::error!(
            "refusing cleartext non-loopback listener {}; configure TLS, or bind nsfyd to \
             loopback when using a same-host TLS reverse proxy",
            listen_addr
        );
        std::process::exit(2);
    }
    if !listen_addr.ip().is_loopback() && cfg.auth_token.is_none() {
        tracing::error!("a non-loopback listener requires --auth-token");
        std::process::exit(2);
    }

    // Burst = one-sixth of the per-minute budget (min 10); refills to the
    // full per-minute rate spread evenly across each second.
    let burst = (cfg.rate_limit_per_min / 6).max(10);
    let refill_per_sec = cfg.rate_limit_per_min as f64 / 60.0;

    let topic_burst = (cfg.topic_rate_limit_per_min / 6).max(10);
    let topic_refill_per_sec = cfg.topic_rate_limit_per_min as f64 / 60.0;

    // Bandwidth burst always fits at least one max-size message, even right
    // after a fresh bucket is created — otherwise a legitimate first message
    // larger than capacity/6 would be rejected outright.
    let bandwidth_burst = (cfg.bandwidth_limit_per_min / 6).max(cfg.max_msg_size as u64);
    let bandwidth_refill_per_sec = cfg.bandwidth_limit_per_min as f64 / 60.0;

    let topic_creation_burst = (cfg.topic_creation_limit_per_min / 6).max(5);
    let topic_creation_refill_per_sec = cfg.topic_creation_limit_per_min as f64 / 60.0;

    let pubsub = PubSub::new(cfg.cache_size, cfg.stream_buffer_size, cfg.max_topics);
    let access =
        access::AccessControl::load(cfg.auth_token.clone(), cfg.topic_access_file.as_deref())
            .unwrap_or_else(|error| {
                tracing::error!("{}", error);
                std::process::exit(2);
            });

    let keep = cfg.db_keep_per_topic.unwrap_or(cfg.cache_size);
    let db = Arc::new(store::Store::open(&cfg.db_path).unwrap_or_else(|error| {
        tracing::error!("failed to open or migrate {}: {}", cfg.db_path, error);
        std::process::exit(2);
    }));
    let topics = tokio::task::spawn_blocking({
        let db = Arc::clone(&db);
        move || db.load_all(keep)
    })
    .await
    .unwrap_or_else(|error| {
        tracing::error!("database load task failed: {}", error);
        std::process::exit(2);
    })
    .unwrap_or_else(|error| {
        tracing::error!("failed to load messages from {}: {}", cfg.db_path, error);
        std::process::exit(2);
    });
    let topic_count = topics.len();
    for (name, messages) in topics {
        pubsub.seed(&name, messages).await;
    }
    info!("loaded {} topic(s) from {}", topic_count, cfg.db_path);
    let persistence = Persistence::sqlite(db).unwrap_or_else(|error| {
        tracing::error!("{}", error);
        std::process::exit(2);
    });
    let db_keep_per_topic = keep;
    let state_store = Arc::new(state_store::StateStore::open(&cfg.db_path).unwrap_or_else(
        |error| {
            tracing::error!("failed to open message state store: {}", error);
            std::process::exit(2);
        },
    ));

    let state = Arc::new(AppState {
        pubsub,
        access,
        rate_limiter: RateLimiter::new(burst as u64, refill_per_sec),
        topic_rate_limiter: RateLimiter::new(topic_burst as u64, topic_refill_per_sec),
        bandwidth_limiter: RateLimiter::new(bandwidth_burst, bandwidth_refill_per_sec),
        topic_creation_limiter: RateLimiter::new(
            topic_creation_burst as u64,
            topic_creation_refill_per_sec,
        ),
        conn_limiter: ConnLimiter::new(cfg.max_conns_per_ip, cfg.max_conns_total),
        max_msg_size: cfg.max_msg_size,
        trust_proxy: cfg.trust_proxy,
        persistence,
        db_keep_per_topic,
        state_store,
        state_hub: state_sync::StateHub::new(),
        state_keep_per_topic: keep.saturating_mul(2).max(500),
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
                state_clone
                    .bandwidth_limiter
                    .sweep(Duration::from_secs(600));
                state_clone
                    .topic_creation_limiter
                    .sweep(Duration::from_secs(600));
                state_clone.conn_limiter.sweep();
            }
        });
    }

    let body_limit = cfg.max_msg_size.saturating_add(4096);
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            HeaderValue::from_static("tauri://localhost"),
            HeaderValue::from_static("http://tauri.localhost"),
            HeaderValue::from_static("http://localhost:1420"),
        ]))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/{topic}", post(handlers::publish))
        .route("/{topic}/ws", get(ws::subscribe))
        .route("/{topic}/state", post(state_sync::update))
        .route("/{topic}/state/ws", get(state_sync::subscribe))
        .route("/{topic}/sse", get(handlers::sse_subscribe))
        .route("/{topic}/json", get(handlers::poll))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            security::guard,
        ))
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Turns a panic in any single request into a 500 response instead of
        // taking down every other client's connection with the process.
        .layer(CatchPanicLayer::new())
        .with_state(state);

    info!("nsfyd listening on {}", cfg.listen);
    info!(
        "cache_size={}, stream_buffer_size={}, max_msg_size={}, max_topics={}, rate_limit_per_min={}, \
         topic_rate_limit_per_min={}, bandwidth_limit_per_min={}, \
         topic_creation_limit_per_min={}, max_conns_per_ip={}, max_conns_total={}, \
         trust_proxy={}, auth={}, persistent={}",
        cfg.cache_size,
        cfg.stream_buffer_size,
        cfg.max_msg_size,
        cfg.max_topics,
        cfg.rate_limit_per_min,
        cfg.topic_rate_limit_per_min,
        cfg.bandwidth_limit_per_min,
        cfg.topic_creation_limit_per_min,
        cfg.max_conns_per_ip,
        cfg.max_conns_total,
        cfg.trust_proxy,
        cfg.auth_token.is_some(),
        true,
    );

    if let (Some(cert), Some(key)) = (&cfg.tls_cert, &cfg.tls_key) {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
            .await
            .unwrap_or_else(|error| {
                tracing::error!("failed to load TLS certificate or key: {}", error);
                std::process::exit(2);
            });
        info!("TLS enabled");
        axum_server::bind_rustls(listen_addr, tls)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    } else {
        let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    }
}
