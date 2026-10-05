use serde::{Deserialize, Serialize};

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_PORT: u16 = 3210;
pub const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    Codex,
    Claude,
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agent::Codex => "codex",
            Agent::Claude => "claude",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Agent::Codex => "Codex",
            Agent::Claude => "Claude Code",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    TaskCompleted,
    PermissionRequired,
    Notification,
    SubagentCompleted,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::TaskCompleted => "task_completed",
            EventType::PermissionRequired => "permission_required",
            EventType::Notification => "notification",
            EventType::SubagentCompleted => "subagent_completed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Success,
    Waiting,
    Info,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Success => "success",
            EventStatus::Waiting => "waiting",
            EventStatus::Info => "info",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            EventStatus::Success => "✅",
            EventStatus::Waiting => "🟡",
            EventStatus::Info => "💬",
        }
    }
}

/// Unified event model (mirrors the Node server's AgentEvent).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub agent: Agent,
    pub event: EventType,
    pub status: EventStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Event as accepted from CLI/hooks (id/timestamp assigned by the server).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventInput {
    pub agent: Agent,
    pub event: EventType,
    #[serde(default)]
    pub status: Option<EventStatus>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    pub title: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Validate + normalize an incoming event into a full AgentEvent. Returns a
/// descriptive error string on invalid input (mirrors Node `normalizeEvent`).
pub fn normalize_event(input: EventInput) -> Result<AgentEvent, String> {
    let status = match input.status {
        Some(s) => s,
        None => default_status(input.event),
    };
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("title is required".to_string());
    }
    let timestamp = input.timestamp.unwrap_or_else(now_ms);
    Ok(AgentEvent {
        id: new_event_id(),
        agent: input.agent,
        event: input.event,
        status,
        project: input.project.filter(|s| !s.is_empty()),
        cwd: input.cwd.filter(|s| !s.is_empty()),
        branch: input.branch.filter(|s| !s.is_empty()),
        title,
        message: input.message.filter(|s| !s.is_empty()),
        timestamp,
        metadata: input.metadata,
    })
}

fn default_status(event: EventType) -> EventStatus {
    match event {
        EventType::PermissionRequired => EventStatus::Waiting,
        EventType::Notification => EventStatus::Info,
        EventType::TaskCompleted | EventType::SubagentCompleted => EventStatus::Success,
    }
}

pub fn new_event_id() -> String {
    format!("evt_{}", random_hex(8))
}

pub fn new_permission_id() -> String {
    format!("pr_{}", random_hex(6))
}

pub fn random_hex(bytes: usize) -> String {
    use rand::RngCore;
    let mut buf = vec![0u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}
