use crate::logger;
use crate::model::AgentEvent;
use async_trait::async_trait;
use std::sync::Arc;

/// A consumer of AgentEvents. Handlers are independent: each one owns a single
/// concern (broadcast, history, notify...) and never knows about the others.
#[async_trait]
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &'static str;
    async fn handle(&self, event: &AgentEvent) -> Result<(), String>;
}

/// Business orchestration: one event in, fan out to every registered handler.
/// A failing handler never breaks the pipeline or the caller (and thus never
/// breaks an Agent).
pub struct Pipeline {
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, handler: Arc<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    pub async fn handle(&self, event: &AgentEvent) {
        for h in &self.handlers {
            if let Err(err) = h.handle(event).await {
                logger::error(
                    &format!("pipeline handler \"{}\" failed (ignored)", h.name()),
                    Some(&err),
                );
            }
        }
    }
}
