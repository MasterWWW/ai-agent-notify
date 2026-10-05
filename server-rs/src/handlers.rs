use crate::config::AppConfig;
use crate::feishu::channels::{feishu_config_from, send_feishu};
use crate::logger;
use crate::model::{AgentEvent, EventStatus};
use crate::pipeline::EventHandler;
use crate::ws::WsHub;
use async_trait::async_trait;
use std::sync::Arc;

/// Broadcast each event to WS clients and log it with the client count.
pub struct WsHandler {
    pub hub: WsHub,
}

#[async_trait]
impl EventHandler for WsHandler {
    fn name(&self) -> &'static str {
        "ws"
    }

    async fn handle(&self, event: &AgentEvent) -> Result<(), String> {
        let clients = self.hub.broadcast(event);
        logger::log(&[
            ("msg", Some("event")),
            ("eventId", Some(&event.id)),
            ("agent", Some(event.agent.as_str())),
            ("event", Some(event.event.as_str())),
            ("status", Some(event.status.as_str())),
            ("clients", Some(&clients.to_string())),
        ]);
        Ok(())
    }
}

/// Notification orchestration: decide whether to notify, pick a channel, send.
pub struct NotifierHandler {
    pub get_config: Arc<dyn Fn() -> AppConfig + Send + Sync>,
}

#[async_trait]
impl EventHandler for NotifierHandler {
    fn name(&self) -> &'static str {
        "notifier"
    }

    async fn handle(&self, event: &AgentEvent) -> Result<(), String> {
        let cfg = (self.get_config)();
        if cfg.feishu_app_id.is_none() && cfg.feishu_webhook.is_none() {
            return Ok(());
        }
        let fcfg = feishu_config_from(&cfg);
        if let Err(e) = send_feishu(&fcfg, event).await {
            logger::error("feishu send failed (ignored)", Some(&e));
        }
        Ok(())
    }
}

/// Convert status icon helpers used by bot rendering.
pub fn status_icon(s: EventStatus) -> &'static str {
    s.icon()
}
