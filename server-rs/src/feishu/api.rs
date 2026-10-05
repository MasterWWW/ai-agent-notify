use std::sync::OnceLock;
use tokio::sync::Mutex;

pub const FEISHU_BASE: &str = "https://open.feishu.cn/open-apis";

struct TokenEntry {
    token: String,
    expires_at: u64,
    app_id: String,
}

static TOKEN_CACHE: OnceLock<Mutex<Option<TokenEntry>>> = OnceLock::new();

pub fn http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Fetch and cache the tenant_access_token (2h TTL, refreshed before expiry).
pub async fn get_tenant_token(app_id: &str, app_secret: &str) -> Result<String, String> {
    let cache = TOKEN_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().await;
    if let Some(entry) = &*guard {
        if entry.app_id == app_id && now_ms() < entry.expires_at.saturating_sub(60_000) {
            return Ok(entry.token.clone());
        }
    }
    let resp = http_client()
        .post(format!("{FEISHU_BASE}/auth/v3/tenant_access_token/internal"))
        .json(&serde_json::json!({ "app_id": app_id, "app_secret": app_secret }))
        .send()
        .await
        .map_err(|e| format!("请求 token 失败: {e}"))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let code = v["code"].as_i64().unwrap_or(-1);
    let token = v["tenant_access_token"].as_str().unwrap_or("").to_string();
    if code != 0 || token.is_empty() {
        return Err(format!(
            "获取 tenant_access_token 失败: code={code} msg={}",
            v["msg"].as_str().unwrap_or("")
        ));
    }
    let expire = v["expire"].as_i64().unwrap_or(7200) as u64;
    *guard = Some(TokenEntry {
        token: token.clone(),
        expires_at: now_ms() + expire * 1000,
        app_id: app_id.to_string(),
    });
    Ok(token)
}

/// Drop the cached token (used when a call reports an expired/invalid token).
pub fn reset_token_cache() {
    if let Some(cache) = TOKEN_CACHE.get() {
        if let Ok(mut guard) = cache.try_lock() {
            *guard = None;
        }
    }
}

/// Fetch the app bot's open_id + name (best-effort, used for @mention detection).
pub async fn get_bot_info(app_id: &str, app_secret: &str) -> Result<(String, String), String> {
    let token = get_tenant_token(app_id, app_secret).await?;
    let resp = http_client()
        .get(format!("{FEISHU_BASE}/bot/v3/info"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(format!("code={} msg={}", v["code"].as_i64().unwrap_or(-1), v["msg"].as_str().unwrap_or("")));
    }
    let open_id = v["bot"]["open_id"].as_str().unwrap_or("").to_string();
    let name = v["bot"]["app_name"].as_str().unwrap_or("").to_string();
    Ok((open_id, name))
}
