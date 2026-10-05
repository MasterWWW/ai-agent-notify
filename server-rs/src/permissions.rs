use crate::config::AppConfig;
use crate::feishu::card::build_permission_card;
use crate::feishu::channels::{send_app_card, update_app_card};
use crate::model::{new_permission_id, Agent};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    Pending,
    Allowed,
    Denied,
    Timeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Serialize)]
pub struct PermissionRequest {
    pub id: String,
    pub agent: Agent,
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub status: PermissionStatus,
    pub created_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<PermissionDecision>,
    #[serde(skip)]
    pub card_message_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PermissionWaitResult {
    pub status: PermissionStatus,
    pub decision: Option<PermissionDecision>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 内存版权限请求存储：pending 请求默认 10 分钟 TTL，超时自动标记 timeout。
/// 长轮询等待通过 waiters 唤醒，不占用线程轮询。
#[derive(Clone)]
pub struct PermissionStore {
    items: Arc<Mutex<HashMap<String, PermissionRequest>>>,
    waiters: Arc<Mutex<HashMap<String, Vec<mpsc::UnboundedSender<PermissionWaitResult>>>>>,
    ttl_ms: u64,
}

impl PermissionStore {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            waiters: Arc::new(Mutex::new(HashMap::new())),
            ttl_ms,
        }
    }

    pub fn create(
        &self,
        agent: Agent,
        tool_name: String,
        command: Option<String>,
        cwd: Option<String>,
        project: Option<String>,
    ) -> PermissionRequest {
        let req = PermissionRequest {
            id: new_permission_id(),
            agent,
            tool_name,
            command,
            cwd,
            project,
            status: PermissionStatus::Pending,
            created_at: now_ms(),
            decided_at: None,
            decision: None,
            card_message_id: None,
        };
        if let Ok(mut items) = self.items.lock() {
            items.insert(req.id.clone(), req.clone());
        }
        req
    }

    pub fn get(&self, id: &str) -> Option<PermissionRequest> {
        self.items.lock().ok().and_then(|g| g.get(id).cloned())
    }

    pub fn list_pending(&self) -> Vec<PermissionRequest> {
        self.items
            .lock()
            .ok()
            .map(|g| g.values().filter(|r| r.status == PermissionStatus::Pending).cloned().collect())
            .unwrap_or_default()
    }

    pub fn resolve(&self, id: &str, decision: PermissionDecision) -> Option<PermissionRequest> {
        let mut items = self.items.lock().ok()?;
        let req = items.get_mut(id)?;
        if req.status != PermissionStatus::Pending {
            return None;
        }
        req.status = if decision == PermissionDecision::Allow {
            PermissionStatus::Allowed
        } else {
            PermissionStatus::Denied
        };
        req.decision = Some(decision);
        req.decided_at = Some(now_ms());
        let req = req.clone();
        drop(items);
        self.wake(id);
        Some(req)
    }

    pub fn mark_timeout(&self, id: &str) -> Option<PermissionRequest> {
        let mut items = self.items.lock().ok()?;
        let req = items.get_mut(id)?;
        if req.status != PermissionStatus::Pending {
            return None;
        }
        req.status = PermissionStatus::Timeout;
        req.decided_at = Some(now_ms());
        let req = req.clone();
        drop(items);
        self.wake(id);
        Some(req)
    }

    /// 长轮询：请求被决定立即返回；超时返回 timeout。
    pub async fn wait(&self, id: &str, timeout_ms: u64) -> PermissionWaitResult {
        {
            let items = self.items.lock().unwrap_or_else(|p| p.into_inner());
            match items.get(id) {
                None => {
                    return PermissionWaitResult {
                        status: PermissionStatus::Timeout,
                        decision: None,
                    }
                }
                Some(r) if r.status != PermissionStatus::Pending => {
                    return PermissionWaitResult {
                        status: r.status,
                        decision: r.decision,
                    }
                }
                Some(_) => {}
            }
        }
        let (tx, mut rx) = mpsc::unbounded_channel::<PermissionWaitResult>();
        if let Ok(mut waiters) = self.waiters.lock() {
            waiters.entry(id.to_string()).or_default().push(tx);
        }
        match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), rx.recv()).await {
            Ok(Some(result)) => result,
            _ => PermissionWaitResult {
                status: PermissionStatus::Timeout,
                decision: None,
            },
        }
    }

    pub fn start_sweep(self: &Arc<Self>, on_timeout: Arc<dyn Fn(PermissionRequest) + Send + Sync>) {
        let store = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await;
            loop {
                interval.tick().await;
                let ids: Vec<String> = {
                    let items = store.items.lock().unwrap_or_else(|p| p.into_inner());
                    let now = now_ms();
                    items
                        .values()
                        .filter(|r| r.status == PermissionStatus::Pending && now - r.created_at > store.ttl_ms)
                        .map(|r| r.id.clone())
                        .collect()
                };
                for id in ids {
                    if let Some(req) = store.mark_timeout(&id) {
                        on_timeout(req);
                    }
                }
            }
        });
    }

    fn wake(&self, id: &str) {
        let list = self.waiters.lock().unwrap_or_else(|p| p.into_inner()).remove(id);
        if let Some(list) = list {
            let result = {
                let items = self.items.lock().unwrap_or_else(|p| p.into_inner());
                match items.get(id) {
                    Some(r) => PermissionWaitResult {
                        status: r.status,
                        decision: r.decision,
                    },
                    None => PermissionWaitResult {
                        status: PermissionStatus::Timeout,
                        decision: None,
                    },
                }
            };
            for tx in list {
                let _ = tx.send(result.clone());
            }
        }
    }
}

// ─── PermissionService: 创建请求 → 发飞书卡片；卡片/文本决定 → 解析 → 唤醒 ──

pub struct PermissionService {
    pub store: Arc<PermissionStore>,
    get_config: Arc<dyn Fn() -> AppConfig + Send + Sync>,
}

impl PermissionService {
    pub fn new(get_config: Arc<dyn Fn() -> AppConfig + Send + Sync>) -> Self {
        Self {
            store: Arc::new(PermissionStore::new(10 * 60_000)),
            get_config,
        }
    }

    pub fn store(&self) -> &Arc<PermissionStore> {
        &self.store
    }

    pub async fn create(
        self: &Arc<Self>,
        agent: Agent,
        tool_name: String,
        command: Option<String>,
        cwd: Option<String>,
    ) -> String {
        let project = cwd
            .as_deref()
            .map(Path::new)
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string());
        let req = self.store.create(agent, tool_name, command, cwd, project);
        let id = req.id.clone();
        // 发卡片失败不影响请求本身（Hook 会等到超时后回退终端审批流）
        let svc = self.clone();
        tokio::spawn(async move {
            if let Err(e) = svc.notify_card(&req).await {
                crate::logger::error("permission card send failed (ignored)", Some(&e));
            }
        });
        id
    }

    pub async fn wait(&self, id: &str, timeout_ms: u64) -> PermissionWaitResult {
        self.store.wait(id, timeout_ms).await
    }

    /// 处理飞书卡片按钮点击（长连接 card.action.trigger）。
    pub async fn handle_card_action(self: &Arc<Self>, message_id: &str, rid: &str, act: &str, operator_open_id: &str) {
        if act != "allow" && act != "deny" {
            return;
        }
        let cfg = (self.get_config)();
        let bound = cfg.feishu_open_id.as_deref().unwrap_or("");
        if bound.is_empty() || operator_open_id != bound {
            crate::logger::log(&[("msg", Some("permission card action ignored (not the bound user)"))]);
            return;
        }
        let Some(req) = self.store.get(rid) else {
            return;
        };
        if req.status != crate::permissions::PermissionStatus::Pending {
            return;
        }
        let decision = if act == "allow" {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny
        };
        self.store.resolve(&req.id, decision);
        // 卡片更新失败不影响决定（决定已生效）
        let svc = Arc::clone(self);
        let req = req.clone();
        let message_id = message_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = svc.update_card(&message_id, &req, Some(decision), "decided").await {
                crate::logger::error("permission card update failed (ignored)", Some(&e));
            }
        });
    }

    /// 文本兜底：用户回复「允许 <id> / 拒绝 <id>」（走消息通道）。
    pub async fn resolve_from_text(self: &Arc<Self>, rid: &str, act: &str, sender_open_id: &str) -> String {
        let cfg = (self.get_config)();
        let bound = cfg.feishu_open_id.as_deref().unwrap_or("");
        if bound.is_empty() || sender_open_id != bound {
            return "⚠️ 只有绑定用户才能操作。".to_string();
        }
        let Some(req) = self.store.get(rid) else {
            return format!("❌ 找不到请求 {rid}。");
        };
        if req.status != PermissionStatus::Pending {
            return format!("ℹ️ 请求 {rid} 已处理（{}）。", status_name(req.status));
        }
        let decision = if act == "allow" {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny
        };
        self.store.resolve(rid, decision);
        if let Some(card_id) = &req.card_message_id {
            let svc = Arc::clone(self);
            let req = req.clone();
            let card_id = card_id.clone();
            tokio::spawn(async move {
                if let Err(e) = svc.update_card(&card_id, &req, Some(decision), "decided").await {
                    crate::logger::error("update card failed", Some(&e));
                }
            });
        }
        if act == "allow" {
            format!("✅ 已允许 {rid}")
        } else {
            format!("⛔ 已拒绝 {rid}")
        }
    }

    /// 请求超时（TTL 到）时把卡片更新为超时状态。
    pub async fn handle_timeout(self: &Arc<Self>, req: PermissionRequest) {
        let Some(card_id) = &req.card_message_id else {
            return;
        };
        let card_id = card_id.clone();
        if let Err(e) = self.update_card(&card_id, &req, None, "timeout").await {
            crate::logger::error("permission timeout card update failed (ignored)", Some(&e));
        }
    }

    async fn notify_card(&self, req: &PermissionRequest) -> Result<(), String> {
        let cfg = (self.get_config)();
        let (Some(app_id), Some(app_secret)) = (&cfg.feishu_app_id, &cfg.feishu_app_secret) else {
            crate::logger::log(&[("msg", Some("permission card skipped (feishu not configured)"))]);
            return Ok(());
        };
        let receive_id = cfg.feishu_open_id.as_ref().or(cfg.feishu_chat_id.as_ref());
        let Some(receive_id) = receive_id else {
            crate::logger::log(&[("msg", Some("permission card skipped (no bound user/chat)"))]);
            return Ok(());
        };
        let card = build_permission_card(req, "pending", None);
        let message_id = send_app_card(app_id, app_secret, receive_id, &card).await?;
        if let Ok(mut items) = self.store.items.lock() {
            if let Some(r) = items.get_mut(&req.id) {
                r.card_message_id = Some(message_id.clone());
            }
        }
        crate::logger::log(&[
            ("msg", Some("permission card sent")),
            ("requestId", Some(&req.id)),
            ("agent", Some(req.agent.as_str())),
            ("tool", Some(&req.tool_name)),
        ]);
        Ok(())
    }

    async fn update_card(
        &self,
        message_id: &str,
        req: &PermissionRequest,
        act: Option<PermissionDecision>,
        state: &str,
    ) -> Result<(), String> {
        let cfg = (self.get_config)();
        let (Some(app_id), Some(app_secret)) = (&cfg.feishu_app_id, &cfg.feishu_app_secret) else {
            return Ok(());
        };
        let card = build_permission_card(req, state, act);
        update_app_card(app_id, app_secret, message_id, &card).await
    }
}

pub fn status_name(s: PermissionStatus) -> &'static str {
    match s {
        PermissionStatus::Pending => "pending",
        PermissionStatus::Allowed => "allowed",
        PermissionStatus::Denied => "denied",
        PermissionStatus::Timeout => "timeout",
    }
}

