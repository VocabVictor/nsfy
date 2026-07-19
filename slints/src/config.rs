use slint::Color;

#[derive(Clone, Debug)]
pub struct TopicConfig {
    pub key: String,
    pub label: String,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: String,
    pub token: Option<String>,
    pub topics: Vec<TopicConfig>,
    pub notifications: bool,
}

impl AppConfig {
    pub fn load() -> Result<Self, String> {
        let server = normalize_server_url(
            &std::env::var("NSFY_SERVER").unwrap_or_else(|_| "http://localhost:8080".into()),
        )?;
        let token = std::env::var("NSFY_AUTH_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());
        let mut topics: Vec<_> = std::env::var("NSFY_TOPICS")
            .unwrap_or_else(|_| "alerts,backups,certificates,deployments,system".into())
            .split(',')
            .filter_map(topic_from_key)
            .collect();
        if topics.is_empty() {
            topics.push(topic_from_key("alerts").expect("built-in topic is valid"));
        }
        let notifications = std::env::var("NSFY_NOTIFICATIONS")
            .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
            .unwrap_or(true);

        Ok(Self {
            server,
            token,
            topics,
            notifications,
        })
    }
}

pub fn normalize_server_url(value: &str) -> Result<String, String> {
    let text = value.trim().trim_end_matches('/');
    let parsed = url::Url::parse(text).map_err(|_| "invalid server URL".to_string())?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("server URL must not contain credentials, a query, or a fragment".into());
    }
    let host = parsed.host_str().ok_or("server URL has no host")?;
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    match parsed.scheme() {
        "https" => Ok(text.to_string()),
        "http" if loopback => Ok(text.to_string()),
        "http" => Err("remote servers must use https://".into()),
        _ => Err("server URL must use https://, or http:// on loopback".into()),
    }
}

pub fn topic_from_key(value: &str) -> Option<TopicConfig> {
    let key = value.trim();
    if key.is_empty()
        || key.len() > 64
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return None;
    }

    let (label, rgb) = match key {
        "alerts" => ("服务器告警", (14, 165, 233)),
        "backups" => ("备份任务", (34, 197, 94)),
        "certificates" => ("证书监控", (245, 158, 11)),
        "deployments" => ("部署通知", (139, 92, 246)),
        "system" => ("系统消息", (20, 184, 166)),
        other => (other, topic_color(other)),
    };
    let (red, green, blue) = rgb;
    let color = Color::from_rgb_u8(red, green, blue);
    Some(TopicConfig {
        key: key.to_owned(),
        label: label.to_owned(),
        color,
    })
}

fn topic_color(key: &str) -> (u8, u8, u8) {
    const COLORS: [(u8, u8, u8); 8] = [
        (239, 68, 68),
        (249, 115, 22),
        (245, 158, 11),
        (34, 197, 94),
        (20, 184, 166),
        (14, 165, 233),
        (59, 130, 246),
        (139, 92, 246),
    ];
    let hash = key.bytes().fold(0_u32, |value, byte| {
        value.wrapping_mul(31) + u32::from(byte)
    });
    COLORS[hash as usize % COLORS.len()]
}

#[cfg(test)]
#[path = "../tests/unit/config.rs"]
mod tests;
