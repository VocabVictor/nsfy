mod config;
mod handlers;
mod message;
mod pubsub;

use axum::{routing::get, routing::post, Router};
use clap::Parser;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use handlers::AppState;
use pubsub::PubSub;

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

    let state = Arc::new(AppState {
        pubsub: PubSub::new(cfg.cache_size),
        auth_token: cfg.auth_token.clone(),
    });

    if cfg.stats_interval > 0 {
        let state_clone = Arc::clone(&state);
        let interval = cfg.stats_interval;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                let stats = state_clone.pubsub.stats();
                info!(
                    "stats: {} topics, {} total subscribers | topics: {:?}",
                    stats.topics, stats.total_subscribers, stats.topic_names
                );
            }
        });
    }

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/{topic}", post(handlers::publish))
        .route("/{topic}/ws", get(handlers::ws_subscribe))
        .route("/{topic}/sse", get(handlers::sse_subscribe))
        .route("/{topic}/json", get(handlers::poll))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.listen).await.unwrap();
    info!("nsfyd listening on {}", cfg.listen);
    info!(
        "cache_size={}, max_msg_size={}, auth={}",
        cfg.cache_size,
        cfg.max_msg_size,
        cfg.auth_token.is_some()
    );

    axum::serve(listener, app).await.unwrap();
}
