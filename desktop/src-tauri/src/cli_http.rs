use crate::config::StoredServer;
use reqwest::blocking::{Client, Response};
use reqwest::header::AUTHORIZATION;
use serde_json::Value;
use std::time::Duration;

pub(super) fn request(
    builder: reqwest::blocking::RequestBuilder,
    server: &StoredServer,
) -> reqwest::blocking::RequestBuilder {
    let token = env_token().or_else(|| server.token.clone());
    match token {
        Some(value) => builder.header(AUTHORIZATION, format!("Bearer {value}")),
        None => builder,
    }
}

pub(super) fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())
}

pub(super) fn print_response(response: Response) -> Result<(), String> {
    let status = response.status();
    let body = response.text().map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("server returned {status}: {}", body.trim()));
    }
    match serde_json::from_str::<Value>(&body) {
        Ok(value) => print_json(&value),
        Err(_) => {
            println!("{body}");
            Ok(())
        }
    }
}

pub(super) fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}

pub(super) fn env_token() -> Option<String> {
    std::env::var("NSFY_AUTH_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
}
