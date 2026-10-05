// HTTP + WebSocket transport (axum). Mirrors Node transport/server.ts.
use crate::logger;
use crate::model::{normalize_event, Agent, EventInput, VERSION};
use crate::permissions::PermissionService;
use crate::pipeline::Pipeline;
use crate::state::token_eq;
use crate::ws::WsHub;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub token: String,
    pub hub: WsHub,
    pub pipeline: Arc<Pipeline>,
    pub permissions: Option<Arc<PermissionService>>,
    pub bot_connected: Arc<std::sync::atomic::AtomicBool>,
}

pub struct RunningApp {
    pub port: u16,
    pub hostname: String,
    pub lan_ips: Vec<String>,
    pub server_task: tokio::task::JoinHandle<()>,
}

impl RunningApp {
    pub async fn close(self) {
        crate::mdns::stop_bonjour();
        self.server_task.abort();
        let _ = self.server_task.await;
    }
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let Some(bearer) = header.strip_prefix("Bearer ").map(|s| s.trim()) else {
        return false;
    };
    token_eq(bearer, &state.token)
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "unauthorized" }))).into_response()
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": VERSION }))
}

async fn create_event(State(st): State<AppState>, headers: HeaderMap, body: String) -> Response {
    if !authorized(&st, &headers) {
        return unauthorized();
    }
    let input: EventInput = match serde_json::from_str(&body) {
        Ok(i) => i,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "invalid json" }))).into_response();
        }
    };
    let event = match normalize_event(input) {
        Ok(e) => e,
        Err(msg) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": msg }))).into_response();
        }
    };
    let event_id = event.id.clone();
    st.pipeline.handle(&event).await;
    (StatusCode::OK, Json(json!({ "success": true, "eventId": event_id }))).into_response()
}

async fn create_permission(State(st): State<AppState>, headers: HeaderMap, body: String) -> Response {
    if !authorized(&st, &headers) {
        return unauthorized();
    }
    let Some(svc) = st.permissions.clone() else {
        return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "error": "not found" }))).into_response();
    };
    let v: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "invalid json" }))).into_response();
        }
    };
    let agent = match v["agent"].as_str() {
        Some("codex") => Agent::Codex,
        Some("claude") => Agent::Claude,
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "invalid permission request" }))).into_response();
        }
    };
    let tool_name = v["toolName"].as_str().unwrap_or("").trim().to_string();
    if tool_name.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "invalid permission request" }))).into_response();
    }
    let command = v["command"].as_str().map(|s| s.chars().take(4000).collect());
    let cwd = v["cwd"].as_str().map(|s| s.chars().take(2000).collect());
    let request_id = svc.create(agent, tool_name, command, cwd).await;
    (StatusCode::OK, Json(json!({ "success": true, "requestId": request_id }))).into_response()
}

async fn wait_permission(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&st, &headers) {
        return unauthorized();
    }
    let Some(svc) = st.permissions.clone() else {
        return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "error": "not found" }))).into_response();
    };
    if id.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "missing request id" }))).into_response();
    }
    let raw = params.get("timeout").and_then(|s| s.parse::<f64>().ok());
    let timeout_ms = raw
        .map(|r| {
            let t = r.trunc() as u64;
            t.clamp(1000, 600_000)
        })
        .unwrap_or(540_000);
    let result = svc.wait(&id, timeout_ms).await;
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "status": status_name(result.status),
            "decision": result.decision.map(|d| decision_name(d)),
        })),
    )
        .into_response()
}

fn status_name(s: crate::permissions::PermissionStatus) -> &'static str {
    match s {
        crate::permissions::PermissionStatus::Pending => "pending",
        crate::permissions::PermissionStatus::Allowed => "allowed",
        crate::permissions::PermissionStatus::Denied => "denied",
        crate::permissions::PermissionStatus::Timeout => "timeout",
    }
}

fn decision_name(d: crate::permissions::PermissionDecision) -> &'static str {
    match d {
        crate::permissions::PermissionDecision::Allow => "allow",
        crate::permissions::PermissionDecision::Deny => "deny",
    }
}

async fn ws_upgrade(
    State(st): State<AppState>,
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let token = params.get("token").cloned().unwrap_or_default();
    if !token_eq(&token, &st.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, st.hub.clone()))
}

async fn handle_socket(mut socket: WebSocket, hub: WsHub) {
    let mut rx = hub.subscribe();
    if socket.send(Message::Text(hub.send_connected().into())).await.is_err() {
        hub.release_client();
        return;
    }
    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        let is_ping = serde_json::from_str::<Value>(t.as_str())
                            .ok()
                            .and_then(|v| v["type"].as_str().map(|s| s == "ping"))
                            .unwrap_or(false);
                        if is_ping {
                            let pong = serde_json::json!({ "type": "pong" }).to_string();
                            if socket.send(Message::Text(pong.into())).await.is_err() { break; }
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
            recv = rx.recv() => {
                match recv {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() { break; }
                    }
                    Err(_) => break,
                }
            }
        }
    }
    hub.release_client();
    let _ = logger::log(&[("msg", Some("ws client disconnected"))]);
}

pub fn router(state: AppState) -> Router {
    let mut r = Router::new()
        .route("/health", get(health))
        .route("/api/events", post(create_event))
        .route("/ws", get(ws_upgrade));
    if state.permissions.is_some() {
        r = r
            .route("/api/permission-requests", post(create_permission))
            .route("/api/permission-requests/{id}/wait", get(wait_permission));
    }
    r.with_state(state)
}

/// Bind and serve. Returns after the listener is bound (server runs in the
/// background task).
pub async fn serve(state: AppState, host: &str, port: u16, mdns: bool) -> Result<RunningApp, String> {
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .map_err(|e| format!("绑定端口失败: {e}"))?;
    let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    let hostname = crate::mdns::get_local_hostname();
    let lan_ips = crate::mdns::get_lan_ips();

    logger::log(&[
        ("msg", Some("server started")),
        ("version", Some(VERSION)),
        ("port", Some(&actual_port.to_string())),
        ("bind", Some(host)),
    ]);
    logger::log(&[
        ("msg", Some("lan hostname (use this on the phone)")),
        ("hostname", Some(&format!("{hostname}:{actual_port}"))),
    ]);
    if lan_ips.is_empty() {
        logger::log(&[("msg", Some("no private LAN ip detected; is the Mac connected to the LAN?"))]);
    } else {
        logger::log(&[("msg", Some("lan ips (for first-time check only)")), ("ips", Some(&lan_ips.join(",")))]);
    }

    if mdns {
        crate::mdns::publish_bonjour_service("AI Task Notify", "ai-task-notify", actual_port);
    }

    let server_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            logger::error("http server error", Some(&e.to_string()));
        }
    });

    Ok(RunningApp {
        port: actual_port,
        hostname,
        lan_ips,
        server_task,
    })
}

/// Extract a Bearer token from an Authorization header (mirrors Node).
pub fn bearer_token(header: Option<&str>) -> Option<String> {
    let h = header?.trim();
    h.strip_prefix("Bearer ")
        .or_else(|| h.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
}

