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

    /// Publish bandwidth budget per client IP, in message bytes per minute.
    /// Separate from --rate-limit-per-min: that one counts requests, this
    /// one counts payload size, so a few maximum-size messages can't sneak
    /// under a request-count-only budget.
    #[arg(long, env = "NSFY_BANDWIDTH_LIMIT_PER_MIN", default_value = "10000000")]
    pub bandwidth_limit_per_min: u64,

    /// How many brand-new topics a single IP may create per minute. Separate
    /// from --max-topics (the global ceiling): this shapes the rate of
    /// creation, not just the total count, so one IP can't claim the whole
    /// topic table's remaining headroom in a burst.
    #[arg(long, env = "NSFY_TOPIC_CREATION_LIMIT_PER_MIN", default_value = "20")]
    pub topic_creation_limit_per_min: u32,

    /// How often to log stats (seconds, 0 = disabled)
    #[arg(long, env = "NSFY_STATS_INTERVAL", default_value = "60")]
    pub stats_interval: u64,

    /// Path to a SQLite database file for persisting messages across
    /// restarts. Unset (the default) means pure in-memory — messages are
    /// lost on restart.
    #[arg(long, env = "NSFY_DB_PATH")]
    pub db_path: Option<String>,

    /// How many messages to retain per topic in the database — same
    /// ring-buffer semantics as --cache-size, just on disk. Defaults to
    /// --cache-size so the database never silently holds more history than
    /// what's already visible through the in-memory replay cache. This is
    /// a per-topic bound, not "keep everything forever": worst-case total
    /// rows is bounded by --max-topics × this value, so size it with that
    /// product in mind.
    #[arg(long, env = "NSFY_DB_KEEP_PER_TOPIC")]
    pub db_keep_per_topic: Option<usize>,
}
