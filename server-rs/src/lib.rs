//! AI Task Notify server (Rust). Library entry: start the server in-process,
//! used by both the standalone CLI and the Tauri desktop shell.

pub mod client;
pub mod config;
pub mod feishu;
pub mod handlers;
pub mod history;
pub mod hooks;
pub mod http;
pub mod logger;
pub mod mdns;
pub mod model;
pub mod permissions;
pub mod pipeline;
pub mod state;
pub mod ws;

pub use model::{DEFAULT_HOST, DEFAULT_PORT, VERSION};

use crate::config::{load_app_config, AppConfig};
use crate::feishu::bot::{handle_bot_message, BotInfo, TextDecider};
use crate::feishu::ws_client::{start_feishu_bot, CardAction, FeishuBotHandle, FeishuBotHandler};
use crate::feishu::NormalizedMessage;
use crate::handlers::{NotifierHandler, WsHandler};
use crate::http::{serve, AppState};
use crate::permissions::PermissionService;
use crate::pipeline::Pipeline;
use crate::ws::WsHub;
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct ServerOptions {
    pub host: String,
    pub port: u16,
    pub token: String,
    pub mdns: bool,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            token: String::new(),
            mdns: true,
        }
    }
}

/// Wire the full handler pipeline and the permission service.
pub fn build_state(token: String) -> AppState {
    let hub = WsHub::new();
    let get_config: Arc<dyn Fn() -> AppConfig + Send + Sync> = Arc::new(load_app_config);

    let mut pipeline = Pipeline::new();
    pipeline.register(Arc::new(WsHandler { hub: hub.clone() }));
    pipeline.register(Arc::new(history::HistoryHandler));
    pipeline.register(Arc::new(NotifierHandler {
        get_config: get_config.clone(),
    }));

    let permission_service = Arc::new(PermissionService::new(get_config));
    {
        let svc = permission_service.clone();
        let store = permission_service.store().clone();
        store.start_sweep(Arc::new(move |req| {
            let svc = svc.clone();
            tokio::spawn(async move {
                svc.handle_timeout(req).await;
            });
        }));
    }

    AppState {
        token,
        hub,
        pipeline: Arc::new(pipeline),
        permissions: Some(permission_service),
        bot_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
}

/// Bind and start the HTTP/WS server (background task). Returns handles.
pub async fn start_server(opts: ServerOptions) -> Result<http::RunningApp, String> {
    let state = build_state(opts.token);
    serve(state, &opts.host, opts.port, opts.mdns).await
}

/// Start the Feishu long-connection bot for the server, wiring message and
/// card-action callbacks to the permission service. Returns None when Feishu
/// is not configured. Failures never stop the server.
pub fn start_feishu_bot_component(state: &AppState) -> Option<FeishuBotHandle> {
    let cfg = load_app_config();
    let (app_id, app_secret) = (cfg.feishu_app_id.clone()?, cfg.feishu_app_secret.clone()?);
    let permissions = state.permissions.clone().expect("permission service exists");
    let handler = AppBotHandler {
        state: state.clone(),
        permissions,
    };
    Some(start_feishu_bot(app_id, app_secret, Arc::new(handler)))
}

struct AppBotHandler {
    state: AppState,
    permissions: Arc<PermissionService>,
}

struct ServiceTextDecider {
    permissions: Arc<PermissionService>,
}

#[async_trait]
impl TextDecider for ServiceTextDecider {
    async fn decide(&self, rid: &str, act: &str, sender_open_id: &str) -> String {
        self.permissions.resolve_from_text(rid, act, sender_open_id).await
    }
}

impl FeishuBotHandler for AppBotHandler {
    fn on_message(&self, msg: NormalizedMessage) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> {
        let permissions = self.permissions.clone();
        let state = self.state.clone();
        Box::pin(async move {
            let connected = state.bot_connected.load(Ordering::Relaxed);
            let bot_info = move || BotInfo { connected };
            let decider = ServiceTextDecider { permissions };
            handle_bot_message(&msg, bot_info, Some(&decider)).await
        })
    }

    fn on_card_action(&self, action: CardAction) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let permissions = self.permissions.clone();
        Box::pin(async move {
            let rid = action.value["rid"].as_str().unwrap_or("").to_string();
            let act = action.value["act"].as_str().unwrap_or("").to_string();
            if !rid.is_empty() && !act.is_empty() {
                permissions
                    .handle_card_action(&action.message_id, &rid, &act, &action.operator_open_id)
                    .await;
            }
        })
    }

    fn on_state(&self, connected: bool) {
        self.state.bot_connected.store(connected, Ordering::Relaxed);
        logger::log(&[("msg", Some("feishu bot state")), ("state", Some(if connected { "connected" } else { "reconnecting" }))]);
    }
}
