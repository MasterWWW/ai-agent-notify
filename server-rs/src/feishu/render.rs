use crate::model::AgentEvent;

/// Render an AgentEvent into the human-readable notification text
/// (mirrors Node render.ts buildText).
pub fn build_text(event: &AgentEvent) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("🤖 {}", event.agent.label()));
    if let Some(p) = &event.project {
        lines.push(p.clone());
    }
    lines.push(format!("{} {}", event.status.icon(), event.title));
    if let Some(m) = &event.message {
        lines.push(m.clone());
    }
    lines.join("\n")
}
