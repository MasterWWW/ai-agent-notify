use crate::permissions::{PermissionDecision, PermissionRequest, PermissionStatus};
use serde_json::{json, Value};

fn agent_label(agent: &str) -> &str {
    match agent {
        "claude" => "Claude",
        _ => "Codex",
    }
}

const CMD_LIMIT: usize = 2000;

fn short_command(command: &str) -> String {
    if command.chars().count() > CMD_LIMIT {
        let cut: String = command.chars().take(CMD_LIMIT).collect();
        format!("{cut}…")
    } else {
        command.to_string()
    }
}

/// Build a Feishu interactive card (mirrors Node card.ts buildPermissionCard).
pub fn build_permission_card(
    request: &PermissionRequest,
    state: &str,
    decision: Option<PermissionDecision>,
) -> Value {
    let agent = agent_label(request.agent.as_str());
    let header = match state {
        "pending" => json!({
            "title": { "tag": "plain_text", "content": format!("⚠️ {agent} 请求执行操作") },
            "template": "orange"
        }),
        "timeout" => json!({
            "title": { "tag": "plain_text", "content": format!("⏰ {agent} 请求已超时") },
            "template": "grey"
        }),
        _ => {
            if decision == Some(PermissionDecision::Allow) {
                json!({ "title": { "tag": "plain_text", "content": format!("✅ {agent} 操作已允许") }, "template": "green" })
            } else {
                json!({ "title": { "tag": "plain_text", "content": format!("⛔ {agent} 操作已拒绝") }, "template": "red" })
            }
        }
    };

    let mut elements: Vec<Value> = Vec::new();
    let mut meta: Vec<String> = Vec::new();
    if let Some(p) = &request.project {
        meta.push(format!("**项目**：{p}"));
    }
    if let Some(c) = &request.cwd {
        meta.push(format!("**目录**：{c}"));
    }
    meta.push(format!("**工具**：{}", request.tool_name));
    elements.push(json!({ "tag": "div", "text": { "tag": "lark_md", "content": meta.join("\n") } }));

    if let Some(cmd) = &request.command {
        elements.push(json!({
            "tag": "div",
            "text": { "tag": "lark_md", "content": format!("```\n{}\n```", short_command(cmd)) }
        }));
    }

    if state == "pending" {
        elements.push(json!({
            "tag": "action",
            "actions": [
                { "tag": "button", "text": { "tag": "plain_text", "content": "允许" }, "type": "primary", "value": { "rid": request.id, "act": "allow" } },
                { "tag": "button", "text": { "tag": "plain_text", "content": "拒绝" }, "type": "danger", "value": { "rid": request.id, "act": "deny" } }
            ]
        }));
    } else if state == "timeout" {
        elements.push(json!({
            "tag": "div",
            "text": { "tag": "lark_md", "content": "⏰ 未在飞书收到决定，请在终端处理该请求。" }
        }));
    } else {
        let content = if decision == Some(PermissionDecision::Allow) {
            "✅ 你已在飞书允许该操作。"
        } else {
            "⛔ 你已在飞书拒绝该操作。"
        };
        elements.push(json!({ "tag": "div", "text": { "tag": "lark_md", "content": content } }));
    }

    json!({
        "config": { "wide_screen_mode": true },
        "header": header,
        "elements": elements
    })
}

/// Status label for display/logging.
pub fn status_str(s: PermissionStatus) -> &'static str {
    match s {
        PermissionStatus::Pending => "pending",
        PermissionStatus::Allowed => "allowed",
        PermissionStatus::Denied => "denied",
        PermissionStatus::Timeout => "timeout",
    }
}
