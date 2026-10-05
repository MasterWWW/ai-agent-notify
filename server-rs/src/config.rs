use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::OpenOptionsExt;

/// Notification app config stored in ~/.ai-task-notify/config.json (0600).
/// Field names mirror the Node/App layer exactly (camelCase).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(rename = "feishuWebhook", default, skip_serializing_if = "Option::is_none")]
    pub feishu_webhook: Option<String>,
    #[serde(rename = "feishuSecret", default, skip_serializing_if = "Option::is_none")]
    pub feishu_secret: Option<String>,
    #[serde(rename = "feishuAppId", default, skip_serializing_if = "Option::is_none")]
    pub feishu_app_id: Option<String>,
    #[serde(rename = "feishuAppSecret", default, skip_serializing_if = "Option::is_none")]
    pub feishu_app_secret: Option<String>,
    #[serde(rename = "feishuChatId", default, skip_serializing_if = "Option::is_none")]
    pub feishu_chat_id: Option<String>,
    #[serde(rename = "feishuOpenId", default, skip_serializing_if = "Option::is_none")]
    pub feishu_open_id: Option<String>,
    #[serde(rename = "feishuMobile", default, skip_serializing_if = "Option::is_none")]
    pub feishu_mobile: Option<String>,
}

pub fn load_app_config() -> AppConfig {
    let f = crate::state::config_file();
    if !f.exists() {
        return AppConfig::default();
    }
    match fs::read_to_string(&f) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_app_config(cfg: &AppConfig) -> std::io::Result<()> {
    let dir = crate::state::state_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(cfg).map_err(std::io::Error::other)?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(crate::state::config_file())?;
    use std::io::Write;
    f.write_all(json.as_bytes())?;
    Ok(())
}
