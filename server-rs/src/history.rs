use crate::model::AgentEvent;
use crate::pipeline::EventHandler;
use async_trait::async_trait;
use std::fs;
use std::io::Write;

const MAX_EVENTS_FILE_SIZE: u64 = 1_000_000;
const KEEP_LINES: usize = 100;

/// Persists events to ~/.ai-task-notify/events.jsonl (for the app UI), capping size.
pub struct HistoryHandler;

#[async_trait]
impl EventHandler for HistoryHandler {
    fn name(&self) -> &'static str {
        "history"
    }

    async fn handle(&self, event: &AgentEvent) -> Result<(), String> {
        let line = serde_json::to_string(event).unwrap_or_default();
        let file = crate::state::events_file();
        if let Ok(meta) = fs::metadata(&file) {
            if meta.len() > MAX_EVENTS_FILE_SIZE {
                if let Ok(content) = fs::read_to_string(&file) {
                    let lines: Vec<&str> = content
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .collect();
                    let tail: Vec<&str> = lines.iter().rev().take(KEEP_LINES).rev().copied().collect();
                    if let Ok(mut f) = fs::File::create(&file) {
                        let _ = writeln!(f, "{}", tail.join("\n"));
                    }
                }
            }
        }
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = writeln!(f, "{line}");
        }
        Ok(())
    }
}

/// Read the last N events from the JSONL history (robust, never throws).
pub fn read_recent_events(n: usize) -> Vec<AgentEvent> {
    let file = crate::state::events_file();
    if !file.exists() {
        return Vec::new();
    }
    let Ok(content) = fs::read_to_string(&file) else {
        return Vec::new();
    };
    let lines: Vec<&str> = content.split('\n').filter(|l| !l.is_empty()).collect();
    let mut out = Vec::new();
    for line in lines.iter().rev().take(n).rev() {
        if let Ok(ev) = serde_json::from_str::<AgentEvent>(line) {
            out.push(ev);
        }
    }
    out
}

/// Short human-readable lines, newest first (used by the app UI / bot).
pub fn recent_lines(limit: usize) -> Vec<String> {
    read_recent_events(limit)
        .iter()
        .rev()
        .map(|e| {
            let agent = if e.agent == crate::model::Agent::Claude {
                "Claude Code"
            } else {
                "Codex"
            };
            let mut line = format!("{} {} · {}", e.status.icon(), agent, e.title);
            if let Some(p) = &e.project {
                line.push_str(" · ");
                line.push_str(p);
            }
            line
        })
        .collect()
}

/// Ensure the events file exists (used by app start, mirrors Swift createFile).
pub fn ensure_events_file() {
    let _ = fs::OpenOptions::new().create(true).append(true).open(crate::state::events_file());
}
