use super::api::{get_tenant_token, http_client, FEISHU_BASE};
use serde_json::Value;

/// 通过手机号查用户的 open_id。需要 contact:user.base:readonly 权限。
pub async fn resolve_open_id_by_mobile(
    app_id: &str,
    app_secret: &str,
    mobile: &str,
) -> Result<String, String> {
    let token = get_tenant_token(app_id, app_secret).await?;
    let resp = http_client()
        .post(format!("{FEISHU_BASE}/contact/v3/users/batch_get_id?user_id_type=open_id"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "mobiles": [mobile] }))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    let v: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(format!("code={} msg={}", v["code"].as_i64().unwrap_or(-1), v["msg"].as_str().unwrap_or("")));
    }
    let user = v["data"]["user_list"].as_array().and_then(|l| l.first());
    let uid = user.and_then(|u| u["user_id"].as_str()).unwrap_or("").to_string();
    if uid.is_empty() {
        return Err("没有找到该手机号对应的用户（请确认手机号在组织通讯录内）".to_string());
    }
    Ok(uid)
}

/// 列出机器人所在的群/会话。需要 im:chat:readonly 权限。
pub async fn list_chats(app_id: &str, app_secret: &str) -> Result<Vec<(String, String)>, String> {
    let token = get_tenant_token(app_id, app_secret).await?;
    let resp = http_client()
        .get(format!("{FEISHU_BASE}/im/v1/chats?page_size=50"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    let v: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(format!("code={} msg={}", v["code"].as_i64().unwrap_or(-1), v["msg"].as_str().unwrap_or("")));
    }
    let mut out = Vec::new();
    if let Some(items) = v["data"]["items"].as_array() {
        for it in items {
            let chat_id = it["chat_id"].as_str().unwrap_or("").to_string();
            let name = it["name"].as_str().unwrap_or("(未命名群)").to_string();
            out.push((chat_id, name));
        }
    }
    Ok(out)
}
