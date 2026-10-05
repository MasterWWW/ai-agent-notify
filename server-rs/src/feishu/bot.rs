use crate::config::load_app_config;
use crate::history::read_recent_events;
use crate::mdns::get_local_hostname;
use crate::model::{AgentEvent, DEFAULT_PORT, VERSION};
use async_trait::async_trait;

/// 机器人连接状态（供 status 命令展示）。
#[derive(Clone, Debug)]
pub struct BotInfo {
    pub connected: bool,
}

/// 文本兜底决定回调：返回给用户的提示文本。
#[async_trait]
pub trait TextDecider: Send + Sync {
    async fn decide(&self, rid: &str, act: &str, sender_open_id: &str) -> String;
}

const HELP: &str = "🤖 AI Task Notify 机器人

可用命令：
ping        测试连通
help        查看帮助
status      查看服务与机器人状态
current     查看当前/最近任务
recent      查看最近 10 条事件

直接发消息即可，不需要 @。";

/// 清洗消息文本：去掉 @提及 / <at> 标签。
pub fn clean_text(raw: &str) -> String {
    // strip <at ...>...</at>
    let mut s = String::new();
    let mut rest = raw;
    loop {
        match rest.find("<at") {
            Some(start) => {
                s.push_str(&rest[..start]);
                match rest[start..].find("</at>") {
                    Some(end) => {
                        rest = &rest[start + end + 5..];
                    }
                    None => {
                        s.push_str(&rest[start..]);
                        break;
                    }
                }
            }
            None => {
                s.push_str(rest);
                break;
            }
        }
    }
    // strip @_user_123 / @word -> " "
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '@' {
            let mut j = i + 1;
            while j < chars.len() && !chars[j].is_whitespace() && chars[j] != '@' {
                j += 1;
            }
            if j > i + 1 {
                out.push(' ');
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out.trim().to_string()
}

/// 取第一个词作为命令（小写）。
pub fn parse_command(raw: &str) -> String {
    clean_text(raw)
        .to_lowercase()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// 解析「允许 pr_xxx / 拒绝 pr_xxx」文本决定。
fn parse_decision(cleaned: &str) -> Option<(String, &'static str)> {
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    let act = match parts[0] {
        "允许" => "allow",
        "拒绝" => "deny",
        _ => return None,
    };
    let rid = parts[1];
    if !rid.starts_with("pr_") || !rid.chars().skip(3).all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some((rid.to_string(), act))
}

fn fmt_time(ts: u64) -> String {
    use chrono::{Local, TimeZone};
    let secs = (ts / 1000) as i64;
    Local
        .timestamp_opt(secs, 0)
        .single()
        .map(|d| d.format("%H:%M:%S").to_string())
        .unwrap_or_default()
}

fn format_event_line(e: &AgentEvent) -> String {
    format!(
        "{} [{}] {}{}{}",
        e.status.icon(),
        fmt_time(e.timestamp),
        e.agent.as_str(),
        e.project.as_ref().map(|p| format!(" {p}")).unwrap_or_default(),
        if e.title.is_empty() {
            String::new()
        } else {
            format!(" {}", e.title)
        }
    )
}

fn build_status(bot: BotInfo) -> String {
    let cfg = load_app_config();
    let last = read_recent_events(1).into_iter().next();
    let mut lines = vec![
        "🤖 AI Task Notify".to_string(),
        String::new(),
        format!("服务：运行中 (v{VERSION})"),
        format!("主机：{}:{DEFAULT_PORT}", get_local_hostname()),
        format!("机器人：{}", if bot.connected { "已连接 ✅" } else { "未连接 ❌" }),
    ];
    if cfg.feishu_chat_id.is_some() || cfg.feishu_open_id.is_some() {
        lines.push("目标：已绑定你的单聊 ✅".to_string());
    } else {
        lines.push("目标：未绑定（先给机器人发一条消息即可自动绑定）".to_string());
    }
    lines.push(match &last {
        Some(e) => format!(
            "最近事件：{} {} {} {}",
            fmt_time(e.timestamp),
            e.agent.as_str(),
            e.event.as_str(),
            e.status.as_str()
        ),
        None => "最近事件：暂无".to_string(),
    });
    lines.join("\n")
}

fn build_current() -> String {
    let events = read_recent_events(20);
    let waiting = events.iter().find(|e| e.status.as_str() == "waiting");
    let target = waiting.or_else(|| events.last());
    match target {
        Some(t) => {
            let head = if waiting.is_some() { "⏳ 当前等待确认" } else { "📌 最近任务" };
            format!("{head}\n\n{}", format_event_line(t))
        }
        None => "当前没有运行中的任务。".to_string(),
    }
}

fn build_recent() -> String {
    let events = read_recent_events(10);
    if events.is_empty() {
        return "还没有事件记录。".to_string();
    }
    let mut lines = vec![format!("📋 最近 {} 条事件", events.len()), String::new()];
    for e in &events {
        lines.push(format_event_line(e));
    }
    lines.join("\n")
}

/// 处理一条发给机器人的消息，返回要回复的文本；返回 None 表示不回复。
/// 私聊消息会自动绑定发送者为通知目标（保存 open_id + chat_id）。
pub async fn handle_bot_message(
    msg: &NormalizedMessage,
    bot_info: impl Fn() -> BotInfo,
    on_text_decision: Option<&dyn TextDecider>,
) -> Option<String> {
    // 只接管私聊；群聊必须 @机器人
    if msg.chat_type != "p2p" && !msg.mentioned_bot {
        return None;
    }

    // 自动绑定：第一次收到私聊就记住 open_id 和 chat_id
    if msg.chat_type == "p2p" && !msg.sender_id.is_empty() {
        let mut cfg = load_app_config();
        if cfg.feishu_app_id.is_some() {
            let mut changed = false;
            if cfg.feishu_open_id.is_none() {
                cfg.feishu_open_id = Some(msg.sender_id.clone());
                changed = true;
            }
            if cfg.feishu_chat_id.is_none() {
                cfg.feishu_chat_id = Some(msg.chat_id.clone());
                changed = true;
            }
            if changed {
                let _ = crate::config::save_app_config(&cfg);
                crate::logger::log(&[
                    ("msg", Some("feishu sender auto-bound")),
                    ("chatId", Some(&msg.chat_id)),
                ]);
            }
        }
    }

    // 文本兜底：允许 <id> / 拒绝 <id>
    if let Some(decider) = on_text_decision {
        let cleaned = clean_text(&msg.content);
        if let Some((rid, act)) = parse_decision(&cleaned) {
            let reply = decider.decide(&rid, act, &msg.sender_id).await;
            return Some(reply);
        }
    }

    let cmd = parse_command(&msg.content);
    let reply = match cmd.as_str() {
        "ping" => "pong".to_string(),
        "help" => HELP.to_string(),
        "status" => build_status(bot_info()),
        "current" => build_current(),
        "recent" => build_recent(),
        other => format!("未识别的命令：{other}\n\n发送 help 查看可用命令。"),
    };
    Some(reply)
}

/// Normalized Feishu message (mirrors the SDK's NormalizedMessage, minimal subset).
#[derive(Clone, Debug)]
pub struct NormalizedMessage {
    pub message_id: String,
    pub chat_id: String,
    pub chat_type: String, // "p2p" | "group"
    pub sender_id: String, // open_id
    pub content: String,
    pub mentioned_bot: bool,
}
