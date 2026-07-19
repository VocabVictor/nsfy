use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredConfig {
    #[serde(default)]
    pub servers: Vec<StoredServer>,
    #[serde(default)]
    pub topics: Vec<StoredTopic>,
    #[serde(default)]
    pub popup_on_notify: bool,
    #[serde(default)]
    pub notification_mode: String,
    #[serde(default = "default_popup_position")]
    pub popup_position: String,
    #[serde(default = "default_layout_mode")]
    pub layout_mode: String,
    #[serde(default = "default_window_behavior")]
    pub window_behavior: String,
    #[serde(default)]
    pub do_not_disturb: bool,
    #[serde(default)]
    pub dnd_allowed_priorities: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredServer {
    pub url: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTopic {
    pub name: String,
    pub server: String,
    #[serde(default)]
    pub unread: u64,
}

impl Default for StoredConfig {
    fn default() -> Self {
        Self {
            servers: vec![StoredServer {
                url: "http://localhost:8080".into(),
                name: "Local".into(),
                token: None,
            }],
            topics: Vec::new(),
            popup_on_notify: false,
            notification_mode: default_notification_mode(),
            popup_position: default_popup_position(),
            layout_mode: default_layout_mode(),
            window_behavior: default_window_behavior(),
            do_not_disturb: false,
            dnd_allowed_priorities: Vec::new(),
        }
    }
}

pub fn config_path() -> Result<PathBuf, String> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".config"))
            })
    };
    base.map(|path| path.join("nsfy").join("config.json"))
        .ok_or_else(|| "cannot determine the user config directory".into())
}

pub fn load_existing() -> Result<Option<StoredConfig>, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(|e| format_path_error(&path, e))?;
    let config = serde_json::from_str(&content)
        .map_err(|e| format!("invalid config {}: {e}", path.display()))?;
    Ok(Some(config))
}

pub fn load() -> Result<StoredConfig, String> {
    Ok(load_existing()?.unwrap_or_default())
}

pub fn save(config: &StoredConfig) -> Result<(), String> {
    for server in &config.servers {
        normalize_url(&server.url)?;
    }
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format_path_error(parent, e))?;
    }
    let content = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| format_path_error(&path, e))
}

pub fn normalize_url(value: &str) -> Result<String, String> {
    let url = value.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(url).map_err(|_| "invalid server URL")?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("credentials are not allowed in server URLs".into());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("server URL must not contain a query or fragment".into());
    }
    let host = parsed.host_str().ok_or("server URL has no host")?;
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    match parsed.scheme() {
        "https" => {}
        "http" if loopback => {}
        "http" => return Err("remote servers must use https://".into()),
        _ => return Err("server URL must use https://, or http:// on loopback".into()),
    }
    Ok(url.to_string())
}

pub fn validate_topic(value: &str) -> Result<String, String> {
    let topic = value.trim();
    if topic.is_empty()
        || topic.len() > 128
        || !topic
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("topic must match [A-Za-z0-9._-] and be at most 128 bytes".into());
    }
    Ok(topic.to_string())
}

pub fn parse_category(value: Option<&str>) -> Result<Vec<String>, String> {
    let Some(value) = value.filter(|item| !item.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    let category: Vec<String> = value
        .split('/')
        .map(str::trim)
        .map(str::to_string)
        .collect();
    if category.len() > 8
        || category.iter().any(|segment| {
            segment.is_empty() || segment.len() > 64 || segment.chars().any(char::is_control)
        })
    {
        return Err("category must contain 1-8 non-empty '/'-separated segments".into());
    }
    Ok(category)
}

fn default_popup_position() -> String {
    "top-right".into()
}

fn default_notification_mode() -> String {
    "system".into()
}

fn default_layout_mode() -> String {
    "split".into()
}

fn default_window_behavior() -> String {
    "resident".into()
}

fn format_path_error(path: &Path, error: std::io::Error) -> String {
    format!("{}: {error}", path.display())
}

#[cfg(test)]
#[path = "../tests/unit/config.rs"]
mod tests;
