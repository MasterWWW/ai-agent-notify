use crate::model::AgentEvent;
use super::api::{get_tenant_token, http_client, FEISHU_BASE};
use super::render::build_text;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Detect receive_id_type from the receive id (mirrors the Node SDK).
pub fn receive_id_type(receive_id: &str) -> &'static str {
    if receive_id.starts_with("ou_") {
        "open_id"
    } else if receive_id.starts_with("oc_") {
        "chat_id"
    } else if receive_id.starts_with("on_") {
        "union_id"
    } else if receive_id.contains('@') {
        "email"
    } else {
        "user_id"
    }
}

/// Send a text message as the self-built app bot.
pub async fn send_app_text(app_id: &str, app_secret: &str, receive_id: &str, text: &str) -> Result<String, String> {
    send_app_message(app_id, app_secret, receive_id, "text", &serde_json::json!({ "text": text })).await
}

/// Send an interactive card message; returns message_id.
pub async fn send_app_card(app_id: &str, app_secret: &str, receive_id: &str, card: &serde_json::Value) -> Result<String, String> {
    send_app_message(app_id, app_secret, receive_id, "interactive", card).await
}

async fn send_app_message(
    app_id: &str,
    app_secret: &str,
    receive_id: &str,
    msg_type: &str,
    content: &serde_json::Value,
) -> Result<String, String> {
    let token = get_tenant_token(app_id, app_secret).await?;
    let id_type = receive_id_type(receive_id);
    let resp = http_client()
        .post(format!("{FEISHU_BASE}/im/v1/messages"))
        .query(&[("receive_id_type", id_type)])
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "receive_id": receive_id,
            "msg_type": msg_type,
            "content": serde_json::to_string(content).unwrap_or_default(),
        }))
        .send()
        .await
        .map_err(|e| format!("发送失败: {e}"))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let code = v["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        return Err(format!("code={code} msg={}", v["msg"].as_str().unwrap_or("")));
    }
    Ok(v["data"]["message_id"].as_str().unwrap_or("").to_string())
}

/// Update an already-sent card message (mirrors channel.updateCard).
pub async fn update_app_card(app_id: &str, app_secret: &str, message_id: &str, card: &serde_json::Value) -> Result<(), String> {
    let token = get_tenant_token(app_id, app_secret).await?;
    // 更新卡片用 PATCH（PUT 会返回 400 field validation failed）
    let resp = http_client()
        .patch(format!("{FEISHU_BASE}/im/v1/messages/{message_id}"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "content": serde_json::to_string(card).unwrap_or_default() }))
        .send()
        .await
        .map_err(|e| format!("更新卡片失败: {e}"))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let code = v["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        return Err(format!("code={code} msg={}", v["msg"].as_str().unwrap_or("")));
    }
    Ok(())
}

/// Custom bot webhook signature: HMAC-SHA256, key = `${timestamp}\n${secret}`.
fn webhook_sign(timestamp: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(format!("{timestamp}\n{secret}").as_bytes()).unwrap();
    mac.update(b"");
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

async fn send_webhook(webhook: &str, secret: Option<&str>, text: &str) -> Result<(), String> {
    let mut body = serde_json::json!({ "msg_type": "text", "content": { "text": text } });
    if let Some(s) = secret {
        if !s.is_empty() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string();
            body["timestamp"] = serde_json::json!(timestamp);
            body["sign"] = serde_json::json!(webhook_sign(&timestamp, s));
        }
    }
    let resp = http_client()
        .post(webhook)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let v: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
    let code = v["code"].as_i64();
    if let Some(code) = code {
        if code != 0 {
            return Err(format!("code={code} msg={}", v["msg"].as_str().unwrap_or("")));
        }
    }
    Ok(())
}

/// Config for the Feishu sender (mirrors Node FeishuConfig).
#[derive(Clone, Debug, Default)]
pub struct FeishuConfig {
    pub webhook: Option<String>,
    pub webhook_secret: Option<String>,
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub chat_id: Option<String>,
    pub open_id: Option<String>,
}

pub fn feishu_config_from(cfg: &crate::config::AppConfig) -> FeishuConfig {
    FeishuConfig {
        webhook: cfg.feishu_webhook.clone(),
        webhook_secret: cfg.feishu_secret.clone(),
        app_id: cfg.feishu_app_id.clone(),
        app_secret: cfg.feishu_app_secret.clone(),
        chat_id: cfg.feishu_chat_id.clone(),
        open_id: cfg.feishu_open_id.clone(),
    }
}

/// Send one event to Feishu. Prefers the self-built app bot, falls back to the
/// custom Webhook. Throws on failure; the caller decides how to record it
/// (notification failures must never block the Agent).
pub async fn send_feishu(cfg: &FeishuConfig, event: &AgentEvent) -> Result<(), String> {
    let text = build_text(event);
    if let (Some(app_id), Some(app_secret)) = (&cfg.app_id, &cfg.app_secret) {
        if let Some(receive_id) = cfg.open_id.as_ref().or(cfg.chat_id.as_ref()) {
            send_app_text(app_id, app_secret, receive_id, &text).await?;
            return Ok(());
        }
    }
    if let Some(webhook) = &cfg.webhook {
        send_webhook(webhook, cfg.webhook_secret.as_deref(), &text).await?;
        return Ok(());
    }
    Err("飞书未配置（需要 App ID/Secret/Chat ID 或 Webhook）".to_string())
}

/// Send a test message to verify the config. Returns a human-readable result.
pub async fn test_feishu(cfg: &FeishuConfig) -> String {
    let text = "✅ AI Task Notify 飞书配置测试成功\n链路正常，可以开始接收任务通知了。";
    if let (Some(app_id), Some(app_secret)) = (&cfg.app_id, &cfg.app_secret) {
        if let Some(receive_id) = cfg.open_id.as_ref().or(cfg.chat_id.as_ref()) {
            return match send_app_text(app_id, app_secret, receive_id, text).await {
                Ok(_) => {
                    if cfg.open_id.is_some() {
                        "✅ 发送成功（自建应用机器人 · 单聊）".to_string()
                    } else {
                        "✅ 发送成功（自建应用机器人 · 群聊）".to_string()
                    }
                }
                Err(e) => format!("❌ 发送失败：{e}"),
            };
        }
    }
    if let Some(webhook) = &cfg.webhook {
        return match send_webhook(webhook, cfg.webhook_secret.as_deref(), text).await {
            Ok(_) => "✅ 发送成功（Webhook 机器人）".to_string(),
            Err(e) => format!("❌ 发送失败：{e}"),
        };
    }
    "❌ 未配置飞书（需要 App ID/Secret + open_id 或 Chat ID，或 Webhook）".to_string()
}
