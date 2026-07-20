use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectResponse {
    ok: bool,
    status: u16,
    body: String,
}

#[tauri::command]
pub async fn post_state_direct(
    server: String,
    topic: String,
    updates: serde_json::Value,
) -> Result<bool, String> {
    Ok(post(
        server,
        topic,
        "state",
        serde_json::json!({ "updates": updates }),
    )
    .await?
    .ok)
}

#[tauri::command]
pub async fn post_message_direct(
    server: String,
    topic: String,
    body: serde_json::Value,
) -> Result<DirectResponse, String> {
    post(server, topic, "", body).await
}

async fn post(
    server: String,
    topic: String,
    suffix: &'static str,
    body: serde_json::Value,
) -> Result<DirectResponse, String> {
    let server = crate::config::normalize_url(&server)?;
    let topic = crate::config::validate_topic(&topic)?;
    let token = crate::config::load()?
        .servers
        .into_iter()
        .find(|item| item.url.trim_end_matches('/') == server)
        .and_then(|item| item.token);
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|error| error.to_string())?;
        let path = if suffix.is_empty() {
            format!("{server}/{topic}")
        } else {
            format!("{server}/{topic}/{suffix}")
        };
        let mut request = client.post(path).json(&body);
        if let Some(token) = token.filter(|value| !value.is_empty()) {
            request = request.bearer_auth(token);
        }
        let response = request.send().map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let ok = response.status().is_success();
        let body = response.text().map_err(|error| error.to_string())?;
        Ok(DirectResponse { ok, status, body })
    })
    .await
    .map_err(|error| error.to_string())?
}
