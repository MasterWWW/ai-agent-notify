use crate::model::{AgentEvent, VERSION};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Server -> client message (mirrors Node ServerMessage).
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Connected {
        #[serde(rename = "serverVersion")]
        server_version: String,
    },
    AgentEvent {
        payload: AgentEvent,
    },
    Pong,
}

/// WebSocket hub: manages clients and broadcasts AgentEvents. No business logic.
#[derive(Clone)]
pub struct WsHub {
    tx: broadcast::Sender<String>,
    clients: Arc<AtomicUsize>,
}

impl WsHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            tx,
            clients: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.clients.fetch_add(1, Ordering::Relaxed);
        self.tx.subscribe()
    }

    pub fn client_count(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }

    pub fn release_client(&self) {
        let prev = self.clients.fetch_sub(1, Ordering::Relaxed);
        if prev == 0 {
            self.clients.store(0, Ordering::Relaxed);
        }
    }

    pub fn send_connected(&self) -> String {
        serde_json::to_string(&ServerMessage::Connected {
            server_version: VERSION.to_string(),
        })
        .unwrap_or_default()
    }

    /// Broadcast an event to every connected client. Returns number delivered.
    pub fn broadcast(&self, event: &AgentEvent) -> usize {
        let msg = serde_json::to_string(&ServerMessage::AgentEvent {
            payload: event.clone(),
        })
        .unwrap_or_default();
        match self.tx.send(msg) {
            Ok(receivers) => receivers,
            Err(_) => 0,
        }
    }

    pub fn connected_snapshot(&self) -> usize {
        self.tx.receiver_count()
    }
}
