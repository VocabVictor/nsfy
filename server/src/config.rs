use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "nsfyd", about = "A minimal high-performance pub-sub notification server")]
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

    /// Optional auth token — clients must pass ?auth=<token>
    #[arg(long, env = "NSFY_AUTH_TOKEN")]
    pub auth_token: Option<String>,

    /// How often to log stats (seconds, 0 = disabled)
    #[arg(long, env = "NSFY_STATS_INTERVAL", default_value = "60")]
    pub stats_interval: u64,
}
