use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::State;

use ai_task_notify_server::config::{load_app_config, save_app_config, AppConfig};
use ai_task_notify_server::history;
use ai_task_notify_server::feishu::FeishuBotHandle;
use ai_task_notify_server::http::RunningApp;

/// Managed app state shared with the Tauri commands: the in-process server
/// state plus handles to the running server and Feishu bot.
pub struct Managed {
    pub server_state: ai_task_notify_server::http::AppState,
    pub server: Mutex<Option<RunningApp>>,
    pub port: Mutex<u16>,
    pub bot: Mutex<Option<FeishuBotHandle>>,
}

#[derive(Serialize)]
pub struct StatusInfo {
    pub running: bool,
    pub port: u16,
    pub hostname: String,
    pub token: String,
    pub feishu_configured: bool,
    pub bot_connected: bool,
    pub recent_events: Vec<String>,
}

#[tauri::command]
pub fn get_status(state: State<Managed>) -> StatusInfo {
    let cfg = load_app_config();
    let running = state.server.lock().map(|g| g.is_some()).unwrap_or(false);
    let port = *state.port.lock().unwrap_or_else(|p| p.into_inner());
    StatusInfo {
        running,
        port,
        hostname: ai_task_notify_server::mdns::get_local_hostname(),
        token: state.server_state.token.clone(),
        feishu_configured: cfg.feishu_app_id.is_some() || cfg.feishu_webhook.is_some(),
        bot_connected: state.server_state.bot_connected.load(Ordering::Relaxed),
        recent_events: history::recent_lines(6),
    }
}

#[tauri::command]
pub fn get_config() -> AppConfig {
    load_app_config()
}

#[derive(Deserialize)]
pub struct FeishuConfigInput {
    #[serde(default)]
    pub feishu_app_id: Option<String>,
    #[serde(default)]
    pub feishu_app_secret: Option<String>,
    #[serde(default)]
    pub feishu_chat_id: Option<String>,
    #[serde(default)]
    pub feishu_webhook: Option<String>,
    #[serde(default)]
    pub feishu_secret: Option<String>,
}

#[tauri::command]
pub async fn save_feishu_config(state: State<'_, Managed>, input: FeishuConfigInput) -> Result<(), String> {
    let mut cfg = load_app_config();
    if let Some(v) = input.feishu_app_id {
        cfg.feishu_app_id = Some(v);
    }
    if let Some(v) = input.feishu_app_secret {
        if !v.trim().is_empty() {
            cfg.feishu_app_secret = Some(v);
        }
    }
    if let Some(v) = input.feishu_chat_id {
        cfg.feishu_chat_id = Some(v);
    }
    if let Some(v) = input.feishu_webhook {
        cfg.feishu_webhook = Some(v);
    }
    if let Some(v) = input.feishu_secret {
        cfg.feishu_secret = Some(v);
    }
    save_app_config(&cfg).map_err(|e| e.to_string())?;
    reload_bot(&state).await;
    Ok(())
}

/// (Re)start the Feishu long-connection bot based on the current config.
pub async fn reload_bot(state: &State<'_, Managed>) {
    let old = state.bot.lock().unwrap().take();
    if let Some(bot) = old {
        bot.stop().await;
    }
    let bot = ai_task_notify_server::start_feishu_bot_component(&state.server_state);
    *state.bot.lock().unwrap() = bot;
}

#[tauri::command]
pub async fn feishu_test() -> Result<String, String> {
    let cfg = load_app_config();
    let fcfg = ai_task_notify_server::feishu::channels::feishu_config_from(&cfg);
    Ok(ai_task_notify_server::feishu::channels::test_feishu(&fcfg).await)
}

#[tauri::command]
pub async fn feishu_chats() -> Result<String, String> {
    let cfg = load_app_config();
    let (Some(app_id), Some(app_secret)) = (&cfg.feishu_app_id, &cfg.feishu_app_secret) else {
        return Err("需要先配置 App ID 和 App Secret".to_string());
    };
    let chats = ai_task_notify_server::feishu::queries::list_chats(app_id, app_secret).await?;
    if chats.is_empty() {
        return Ok("（没有找到机器人所在的群）".to_string());
    }
    Ok(chats
        .iter()
        .map(|(id, name)| format!("{id}\t{name}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_state_dir() -> Result<(), String> {
    let dir = ai_task_notify_server::state::state_dir();
    open::that(dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_server(state: State<'_, Managed>, port: u16) -> Result<u16, String> {
    if state.server.lock().map(|g| g.is_some()).unwrap_or(false) {
        return Ok(*state.port.lock().unwrap_or_else(|p| p.into_inner()));
    }
    let server_state = state.server_state.clone();
    let running: RunningApp =
        ai_task_notify_server::http::serve(server_state, "0.0.0.0", port, true).await?;
    let p = running.port;
    *state.port.lock().unwrap() = p;
    *state.server.lock().unwrap() = Some(running);
    Ok(p)
}

#[tauri::command]
pub async fn stop_server(state: State<'_, Managed>) -> Result<(), String> {
    let app = state.server.lock().unwrap().take();
    if let Some(app) = app {
        app.close().await;
    }
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
