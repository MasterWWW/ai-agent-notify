// HTTP client helpers for the CLI (status / test / hook forwarding).
use crate::model::EventInput;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ClientOptions {
    pub base_url: String,
    pub token: String,
}

pub fn resolve_base_url(cli: Option<String>) -> String {
    if let Some(u) = cli {
        if !u.is_empty() {
            return u.trim_end_matches('/').to_string();
        }
    }
    if let Ok(env) = std::env::var("AI_TASK_NOTIFY_BASE_URL") {
        if !env.is_empty() {
            return env.trim_end_matches('/').to_string();
        }
    }
    "http://127.0.0.1:3210".to_string()
}

fn http_client() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(35))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

pub async fn get_health(base_url: &str) -> Result<(String, String), String> {
    let resp = http_client()
        .get(format!("{base_url}/health"))
        .send()
        .await
        .map_err(|e| format!("health check failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("health check failed: HTTP {}", resp.status().as_u16()));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok((
        v["status"].as_str().unwrap_or("?").to_string(),
        v["version"].as_str().unwrap_or("?").to_string(),
    ))
}

pub async fn post_event(opts: &ClientOptions, input: &EventInput) -> Result<String, String> {
    let resp = http_client()
        .post(format!("{}/api/events", opts.base_url))
        .bearer_auth(&opts.token)
        .json(input)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if status != 200 || v["success"].as_bool() != Some(true) {
        let fallback = format!("HTTP {status}");
        let msg = v["error"].as_str().unwrap_or(&fallback);
        return Err(msg.to_string());
    }
    Ok(v["eventId"].as_str().unwrap_or("unknown").to_string())
}

pub async fn post_permission_request(
    opts: &ClientOptions,
    input: &crate::hooks::PermissionCreateInput,
) -> Result<String, String> {
    let body = serde_json::json!({
        "agent": input.agent.as_str(),
        "toolName": input.tool_name,
        "command": input.command,
        "cwd": input.cwd,
    });
    let resp = http_client()
        .post(format!("{}/api/permission-requests", opts.base_url))
        .bearer_auth(&opts.token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if status != 200 || v["success"].as_bool() != Some(true) {
        let fallback = format!("HTTP {status}");
        let msg = v["error"].as_str().unwrap_or(&fallback);
        return Err(msg.to_string());
    }
    v["requestId"].as_str().map(|s| s.to_string()).ok_or_else(|| "missing requestId".to_string())
}

pub async fn wait_permission_decision(
    opts: &ClientOptions,
    request_id: &str,
    timeout_ms: u64,
) -> Result<(String, Option<String>), String> {
    let url = format!(
        "{}/api/permission-requests/{}/wait?timeout={}",
        opts.base_url, request_id, timeout_ms
    );
    let resp = http_client()
        .get(url)
        .bearer_auth(&opts.token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if status != 200 || v["success"].as_bool() != Some(true) {
        let fallback = format!("HTTP {status}");
        let msg = v["error"].as_str().unwrap_or(&fallback);
        return Err(msg.to_string());
    }
    let status = v["status"].as_str().unwrap_or("timeout").to_string();
    let decision = v["decision"].as_str().map(|s| s.to_string());
    Ok((status, decision))
}
