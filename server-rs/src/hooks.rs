// Hook payload mapping (Codex / Claude) -> unified AgentEvent.
// Mirrors Node transport/hooks.ts.
use crate::model::{Agent, EventInput, EventStatus, EventType};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookKind {
    CodexStop,
    CodexPermission,
    CodexSubagentStop,
    ClaudeStop,
    ClaudePermission,
    ClaudeNotification,
}

impl HookKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "codex-stop" => Some(HookKind::CodexStop),
            "codex-permission" => Some(HookKind::CodexPermission),
            "codex-subagent-stop" => Some(HookKind::CodexSubagentStop),
            "claude-stop" => Some(HookKind::ClaudeStop),
            "claude-permission" => Some(HookKind::ClaudePermission),
            "claude-notification" => Some(HookKind::ClaudeNotification),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HookKind::CodexStop => "codex-stop",
            HookKind::CodexPermission => "codex-permission",
            HookKind::CodexSubagentStop => "codex-subagent-stop",
            HookKind::ClaudeStop => "claude-stop",
            HookKind::ClaudePermission => "claude-permission",
            HookKind::ClaudeNotification => "claude-notification",
        }
    }

    pub fn all() -> &'static [HookKind] {
        &[
            HookKind::CodexStop,
            HookKind::CodexPermission,
            HookKind::CodexSubagentStop,
            HookKind::ClaudeStop,
            HookKind::ClaudePermission,
            HookKind::ClaudeNotification,
        ]
    }

    pub fn agent(&self) -> Agent {
        match self {
            HookKind::CodexStop
            | HookKind::CodexPermission
            | HookKind::CodexSubagentStop => Agent::Codex,
            _ => Agent::Claude,
        }
    }

    pub fn is_permission(&self) -> bool {
        matches!(self, HookKind::CodexPermission | HookKind::ClaudePermission)
    }
}

fn project_of(input: &Value) -> Option<String> {
    if let Some(p) = input["project"].as_str() {
        if !p.is_empty() {
            return Some(p.to_string());
        }
    }
    if let Some(cwd) = input["cwd"].as_str() {
        if !cwd.is_empty() {
            let base = cwd.rsplit('/').next().unwrap_or(cwd);
            if !base.is_empty() {
                return Some(base.to_string());
            }
        }
    }
    None
}

fn first_string(vals: &[&Value]) -> Option<String> {
    for v in vals {
        if let Some(s) = v.as_str() {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn summarize(value: &Value, max: usize) -> Option<String> {
    let s = if let Some(v) = value.as_str() {
        v.trim().to_string()
    } else if value.is_object() || value.is_array() {
        serde_json::to_string(value).unwrap_or_default()
    } else {
        value.to_string()
    };
    if s.is_empty() {
        return None;
    }
    if s.chars().count() > max {
        let cut: String = s.chars().take(max).collect();
        Some(format!("{cut}…"))
    } else {
        Some(s)
    }
}

fn extract_codex_permission_message(input: &Value) -> Option<String> {
    if let Some(perms) = input["permissions"].as_array() {
        if !perms.is_empty() {
            let mut parts: Vec<String> = Vec::new();
            for p in perms.iter().take(3) {
                let part = first_string(&[&p["command"], &p["question"], &p["explanation"], &p["kind"]])
                    .or_else(|| summarize(p, 200));
                if let Some(part) = part {
                    parts.push(part);
                }
            }
            if !parts.is_empty() {
                let joined = parts.join(" | ");
                let cut: String = joined.chars().take(300).collect();
                return Some(cut);
            }
        }
    }
    first_string(&[&input["reason"], &input["question"], &input["explanation"]])
        .or_else(|| summarize(&input["payload"], 200))
}

fn extract_claude_permission_message(input: &Value) -> Option<String> {
    let pr = &input["permission_request"];
    if pr.is_object() {
        return first_string(&[&pr["question"], &pr["permission"], &pr["action"], &pr["message"]])
            .or_else(|| summarize(pr, 200));
    }
    first_string(&[&input["question"], &input["message"], &input["action"]])
        .or_else(|| summarize(&input["payload"], 200))
}

fn extract_claude_notification_message(input: &Value) -> (String, Option<String>) {
    if let Some(m) = input["message"].as_str() {
        let s = m.trim();
        if !s.is_empty() {
            if s.chars().count() > 60 {
                let cut: String = s.chars().take(60).collect();
                return ("Claude Code 通知".to_string(), Some(format!("{cut}…")));
            }
            return ("Claude Code 通知".to_string(), Some(s.to_string()));
        }
    }
    let title = first_string(&[&input["title"]]).unwrap_or_else(|| "Claude Code 通知".to_string());
    (title, None)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Map a raw hook stdin payload to a unified event input.
pub fn map_hook_to_event(kind: HookKind, input: &Value) -> EventInput {
    let cwd = input["cwd"].as_str().filter(|s| !s.is_empty()).map(|s| s.to_string());
    let project = project_of(input);
    match kind {
        HookKind::CodexStop | HookKind::ClaudeStop => EventInput {
            agent: kind.agent(),
            event: EventType::TaskCompleted,
            status: Some(EventStatus::Success),
            title: match kind {
                HookKind::CodexStop => "Codex 任务完成".to_string(),
                _ => "Claude Code 任务完成".to_string(),
            },
            message: first_string(&[&input["summary"], &input["result"]]),
            project,
            cwd,
            branch: None,
            timestamp: Some(now_ms()),
            metadata: None,
        },
        HookKind::CodexPermission | HookKind::ClaudePermission => EventInput {
            agent: kind.agent(),
            event: EventType::PermissionRequired,
            status: Some(EventStatus::Waiting),
            title: match kind {
                HookKind::CodexPermission => "Codex 等待权限".to_string(),
                _ => "Claude Code 等待权限".to_string(),
            },
            message: if kind == HookKind::CodexPermission {
                extract_codex_permission_message(input)
            } else {
                extract_claude_permission_message(input)
            },
            project,
            cwd,
            branch: None,
            timestamp: Some(now_ms()),
            metadata: None,
        },
        HookKind::CodexSubagentStop => EventInput {
            agent: Agent::Codex,
            event: EventType::SubagentCompleted,
            status: Some(EventStatus::Success),
            title: "Codex 子任务完成".to_string(),
            message: first_string(&[&input["summary"], &input["result"]]),
            project,
            cwd,
            branch: None,
            timestamp: Some(now_ms()),
            metadata: None,
        },
        HookKind::ClaudeNotification => {
            let (title, message) = extract_claude_notification_message(input);
            EventInput {
                agent: Agent::Claude,
                event: EventType::Notification,
                status: Some(EventStatus::Info),
                title,
                message,
                project,
                cwd,
                branch: None,
                timestamp: Some(now_ms()),
                metadata: None,
            }
        }
    }
}

/// 安全命令自动放行：只读、无副作用的命令不发卡片、直接 allow。
pub fn is_safe_command(cmd: &str) -> bool {
    if cmd.is_empty() {
        return false;
    }
    let c: String = cmd.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    const SAFE_SIMPLE: &[&str] = &[
        "ls", "pwd", "whoami", "date", "uname", "uptime", "which", "type", "pwd -P",
    ];
    if SAFE_SIMPLE.contains(&c.as_str()) {
        return true;
    }
    if c.starts_with("echo ") {
        return true;
    }
    let git_safe = [
        "git status", "git diff", "git log", "git show", "git branch", "git remote",
        "git tag", "git stash list", "git config --get",
    ];
    for prefix in git_safe {
        if c == prefix || c.starts_with(&format!("{prefix} ")) {
            return true;
        }
    }
    false
}

/// 从 Hook 事件 JSON 提取权限请求（Codex/Claude PermissionRequest 事件）。
pub fn extract_permission_input(agent: Agent, input: &Value, fallback_cwd: &str) -> PermissionCreateInput {
    let tool_name = input["tool_name"]
        .as_str()
        .or_else(|| input["toolName"].as_str())
        .unwrap_or("unknown")
        .chars()
        .take(100)
        .collect::<String>();
    let tool_input = if input["tool_input"].is_object() {
        &input["tool_input"]
    } else if input["toolInput"].is_object() {
        &input["toolInput"]
    } else {
        &Value::Null
    };
    let command = tool_input["command"].as_str().map(|s| s.to_string());
    let description = tool_input["description"].as_str().map(|s| s.to_string());
    let cwd = input["cwd"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_cwd.to_string());
    PermissionCreateInput {
        agent,
        tool_name,
        command: command.or(description),
        cwd: Some(cwd),
    }
}

#[derive(Clone, Debug)]
pub struct PermissionCreateInput {
    pub agent: Agent,
    pub tool_name: String,
    pub command: Option<String>,
    pub cwd: Option<String>,
}
