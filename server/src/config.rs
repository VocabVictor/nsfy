use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "nsfyd",
    about = "A minimal high-performance pub-sub notification server"
)]
pub struct Config {
    /// Address to listen on
    #[arg(long, env = "NSFY_LISTEN", default_value = "0.0.0.0:8080")]
    pub listen: String,

    /// Max cached messages per topic
    #[arg(long, env = "NSFY_CACHE_SIZE", default_value = "100")]
    pub cache_size: usize,

    /// Max message body size in bytes
    #[arg(long, env = "NSFY_MAX_MSG_SIZE", default_value = "65536")]
    pub max_msg_size: usize,

    /// Optional auth token — clients must pass Authorization: Bearer <token>
    /// or ?auth=<token>. Required on every route, including `/`, once set.
    #[arg(long, env = "NSFY_AUTH_TOKEN")]
    pub auth_token: Option<String>,

    /// Max requests per minute per client IP (burst + sustained refill)
    #[arg(long, env = "NSFY_RATE_LIMIT_PER_MIN", default_value = "300")]
    pub rate_limit_per_min: u32,

    /// Max distinct topics the server will track at once
    #[arg(long, env = "NSFY_MAX_TOPICS", default_value = "10000")]
    pub max_topics: usize,

    /// Aggregate publish budget per topic (requests/min), across all IPs —
    /// catches a distributed flood that per-IP limiting alone can't
    #[arg(long, env = "NSFY_TOPIC_RATE_LIMIT_PER_MIN", default_value = "1200")]
    pub topic_rate_limit_per_min: u32,

    /// Max concurrent WS/SSE connections per client IP
    #[arg(long, env = "NSFY_MAX_CONNS_PER_IP", default_value = "20")]
    pub max_conns_per_ip: u32,

    /// Max concurrent WS/SSE connections server-wide
    #[arg(long, env = "NSFY_MAX_CONNS_TOTAL", default_value = "10000")]
    pub max_conns_total: u32,

    /// Trust X-Forwarded-For / X-Real-IP for the client IP used in rate
    /// limiting. Only enable when nsfyd sits behind a reverse proxy you
    /// control that overwrites these headers — otherwise a direct client
    /// can forge them and dodge its own rate limit.
    #[arg(long, env = "NSFY_TRUST_PROXY", default_value_t = false)]
    pub trust_proxy: bool,

    /// How often to log stats (seconds, 0 = disabled)
    #[arg(long, env = "NSFY_STATS_INTERVAL", default_value = "60")]
    pub stats_interval: u64,
}
