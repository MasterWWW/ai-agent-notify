// Feishu long connection (WebSocket) client, reimplemented from the
// @larksuiteoapi/node-sdk protocol without the Node dependency.
use super::api;
use super::bot::{clean_text, NormalizedMessage};
use super::ws_proto::{decode_frame, Frame, Header};
use crate::logger;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

// ─── callbacks ──────────────────────────────────────────────────────────────

pub struct CardAction {
    pub message_id: String,
    pub chat_id: String,
    pub operator_open_id: String,
    pub value: Value,
}

pub trait FeishuBotHandler: Send + Sync {
    fn on_message(&self, msg: NormalizedMessage) -> Pin<Box<dyn Future<Output = Option<String>> + Send>>;
    fn on_card_action(&self, action: CardAction) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move { let _ = action; })
    }
    fn on_state(&self, connected: bool) {
        let _ = connected;
    }
}

pub struct FeishuBotHandle {
    cancel: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}

impl FeishuBotHandle {
    pub async fn stop(self) {
        self.cancel.cancel();
        let _ = self.task.await;
    }
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }
}

// ─── endpoint discovery ─────────────────────────────────────────────────────

struct EndpointConfig {
    url: String,
    service_id: i64,
    ping_interval_ms: u64,
    reconnect_interval_ms: u64,
    reconnect_nonce_ms: u64,
}

fn parse_query_number(url_str: &str, key: &str) -> Option<i64> {
    let url = url::Url::parse(url_str).ok()?;
    url.query_pairs()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse().ok())
}

// The long-connection endpoint discovery lives under the bare domain
// (https://open.feishu.cn/callback/ws/endpoint), NOT under /open-apis.
const FEISHU_WS_ENDPOINT: &str = "https://open.feishu.cn/callback/ws/endpoint";

async fn fetch_endpoint(app_id: &str, app_secret: &str) -> Result<EndpointConfig, String> {
    let resp = api::http_client()
        .post(FEISHU_WS_ENDPOINT)
        .header("locale", "zh")
        .header("User-Agent", "ai-task-notify/0.1.0 (rust)")
        .json(&serde_json::json!({ "AppID": app_id, "AppSecret": app_secret }))
        .send()
        .await
        .map_err(|e| format!("endpoint 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("endpoint 读取响应失败: {e}"))?;
    if status != 200 {
        return Err(format!("endpoint HTTP {status}: {text}"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("endpoint 解析失败 ({status}): {e} body={}", &text[..text.len().min(200)]))?;
    let code = v["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        return Err(format!("code={code} msg={}", v["msg"].as_str().unwrap_or("")));
    }
    let url = v["data"]["URL"].as_str().unwrap_or("").to_string();
    if url.is_empty() {
        return Err("endpoint URL 为空".to_string());
    }
    let cc = &v["data"]["ClientConfig"];
    let service_id = parse_query_number(&url, "service_id").unwrap_or(0);
    Ok(EndpointConfig {
        url,
        service_id,
        ping_interval_ms: cc["PingInterval"].as_u64().unwrap_or(120) * 1000,
        reconnect_interval_ms: cc["ReconnectInterval"].as_u64().unwrap_or(120) * 1000,
        reconnect_nonce_ms: cc["ReconnectNonce"].as_u64().unwrap_or(30) * 1000,
    })
}

// ─── event fragment merging ─────────────────────────────────────────────────

#[derive(Default)]
struct FragmentBuffer {
    sum: usize,
    parts: Vec<Option<Vec<u8>>>,
}

fn merge_fragment(
    buffers: &mut HashMap<String, FragmentBuffer>,
    message_id: &str,
    sum: usize,
    seq: usize,
    payload: Vec<u8>,
) -> Option<Value> {
    if sum == 0 || seq >= sum {
        return None;
    }
    if sum == 1 && seq == 0 {
        return serde_json::from_slice(&payload).ok();
    }
    let buf = buffers.entry(message_id.to_string()).or_insert_with(|| FragmentBuffer {
        sum,
        parts: vec![None; sum],
    });
    if buf.sum != sum {
        return None;
    }
    buf.parts[seq] = Some(payload);
    if buf.parts.iter().any(|p| p.is_none()) {
        return None;
    }
    let mut merged = Vec::new();
    for p in &buf.parts {
        if let Some(bytes) = p {
            merged.extend_from_slice(bytes);
        }
    }
    buffers.remove(message_id);
    serde_json::from_slice(&merged).ok()
}

// ─── event normalization ────────────────────────────────────────────────────

fn extract_text_content(content_raw: &str, msg_type: &str) -> String {
    if msg_type == "text" {
        if let Ok(v) = serde_json::from_str::<Value>(content_raw) {
            if let Some(text) = v["text"].as_str() {
                return clean_text(text);
            }
        }
    }
    String::new()
}

fn detect_mentioned_bot(message: &Value, bot_open_id: &Arc<Mutex<Option<String>>>) -> bool {
    let bot_id = bot_open_id.lock().ok().and_then(|g| g.clone()).unwrap_or_default();
    if let Some(mentions) = message["mentions"].as_array() {
        for m in mentions {
            if let Some(id) = m["id"]["open_id"].as_str() {
                if !bot_id.is_empty() && id == bot_id {
                    return true;
                }
            }
        }
    }
    false
}

fn normalize_message(evt: &Value, bot_open_id: &Arc<Mutex<Option<String>>>) -> Option<NormalizedMessage> {
    let message = evt.get("message")?;
    let message_id = message["message_id"].as_str()?.to_string();
    let chat_id = message["chat_id"].as_str()?.to_string();
    let chat_type = message["chat_type"].as_str().unwrap_or("group").to_string();
    let content_raw = message["content"].as_str().unwrap_or("");
    let msg_type = message["message_type"].as_str().unwrap_or("text");
    let sender = evt.get("sender")?.get("sender_id")?;
    let sender_id = sender["open_id"]
        .as_str()
        .or_else(|| sender["user_id"].as_str())
        .or_else(|| sender["union_id"].as_str())?
        .to_string();
    let mentioned_bot = detect_mentioned_bot(message, bot_open_id);
    Some(NormalizedMessage {
        message_id,
        chat_id,
        chat_type,
        sender_id,
        content: extract_text_content(content_raw, msg_type),
        mentioned_bot,
    })
}

fn normalize_card_action(evt: &Value) -> Option<CardAction> {
    let message_id = evt["context"]["open_message_id"]
        .as_str()
        .or_else(|| evt["open_message_id"].as_str())?
        .to_string();
    let chat_id = evt["context"]["open_chat_id"]
        .as_str()
        .or_else(|| evt["open_chat_id"].as_str())?
        .to_string();
    let operator_open_id = evt["operator"]["open_id"].as_str()?.to_string();
    let value = evt["action"]["value"].clone();
    Some(CardAction {
        message_id,
        chat_id,
        operator_open_id,
        value,
    })
}

// ─── connection loop ────────────────────────────────────────────────────────

async fn run_connection(
    cfg: &EndpointConfig,
    app_id: &str,
    app_secret: &str,
    handler: &Arc<dyn FeishuBotHandler>,
    bot_open_id: &Arc<Mutex<Option<String>>>,
    cancel: &CancellationToken,
) -> Result<(), String> {
    let (ws, _) = tokio_tungstenite::connect_async(&cfg.url)
        .await
        .map_err(|e| format!("ws connect failed: {e}"))?;
    let (mut sink, mut stream) = ws.split();
    let (tx_out, mut rx_out) = mpsc::unbounded_channel::<Message>();

    let writer = tokio::spawn(async move {
        while let Some(msg) = rx_out.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    handler.on_state(true);
    logger::log(&[("msg", Some("feishu bot ready (长连接已建立)")), ("serviceId", Some(&cfg.service_id.to_string()))]);

    // ping loop
    let ping_interval = cfg.ping_interval_ms.max(5000);
    let service_id = cfg.service_id;
    let ping_tx = tx_out.clone();
    let ping_cancel = cancel.clone();
    let ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(ping_interval));
        interval.tick().await; // first tick fires immediately
        loop {
            tokio::select! {
                _ = ping_cancel.cancelled() => break,
                _ = interval.tick() => {
                    let frame = Frame {
                        headers: vec![Header { key: "type".to_string(), value: "ping".to_string() }],
                        service: service_id as i32,
                        method: 0,
                        ..Default::default()
                    };
                    if ping_tx.send(Message::Binary(frame.encode())).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let result = message_loop(&mut stream, &tx_out, app_id, app_secret, handler, bot_open_id, cancel).await;

    ping_task.abort();
    drop(tx_out);
    let _ = writer.await;
    result
}

async fn message_loop<S>(
    stream: &mut S,
    tx_out: &mpsc::UnboundedSender<Message>,
    app_id: &str,
    app_secret: &str,
    handler: &Arc<dyn FeishuBotHandler>,
    bot_open_id: &Arc<Mutex<Option<String>>>,
    cancel: &CancellationToken,
) -> Result<(), String>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let mut buffers: HashMap<String, FragmentBuffer> = HashMap::new();
    loop {
        let item = tokio::select! {
            _ = cancel.cancelled() => return Err("cancelled".to_string()),
            item = stream.next() => item,
        };
        let Some(msg) = item else {
            return Err("ws stream ended".to_string());
        };
        let msg = msg.map_err(|e| e.to_string())?;
        let bytes = match msg {
            Message::Binary(b) => b,
            Message::Text(t) => t.into_bytes(),
            Message::Close(_) => return Err("ws closed by peer".to_string()),
            _ => continue,
        };
        let frame = match decode_frame(&bytes) {
            Ok(f) => f,
            Err(_) => continue,
        };
        if frame.method == 0 {
            // control frame: ignore server pings; absorb pong config updates
            continue;
        }
        if frame.method != 1 {
            continue;
        }
        let headers = frame.header_map();
        let msg_type = headers.get("type").cloned().unwrap_or_default();
        if msg_type != "event" {
            continue;
        }
        let message_id = headers.get("message_id").cloned().unwrap_or_default();
        let sum: usize = headers.get("sum").and_then(|s| s.parse().ok()).unwrap_or(1);
        let seq: usize = headers.get("seq").and_then(|s| s.parse().ok()).unwrap_or(0);
        let payload = frame.payload.clone().unwrap_or_default();
        let Some(evt_json) = merge_fragment(&mut buffers, &message_id, sum, seq, payload) else {
            continue;
        };

        let event_type = evt_json["header"]["event_type"].as_str().unwrap_or("").to_string();
        let evt = &evt_json["event"];
        if event_type == "im.message.receive_v1" {
            if let Some(nmsg) = normalize_message(evt, bot_open_id) {
                let reply = handler.on_message(nmsg.clone()).await;
                if let Some(text) = reply {
                    if let Err(e) =
                        super::channels::send_app_text(app_id, app_secret, &nmsg.chat_id, &text).await
                    {
                        logger::error("feishu bot reply failed", Some(&e));
                    }
                }
            }
        } else if event_type == "card.action.trigger" {
            if let Some(action) = normalize_card_action(evt) {
                handler.on_card_action(action).await;
            }
        }

        // ack
        let mut ack_headers = frame.headers.clone();
        ack_headers.push(Header {
            key: "biz_rt".to_string(),
            value: "0".to_string(),
        });
        let ack = Frame {
            seq_id: frame.seq_id,
            log_id: frame.log_id,
            service: frame.service,
            method: frame.method,
            headers: ack_headers,
            payload: Some(br#"{"code":200}"#.to_vec()),
            ..Default::default()
        };
        let _ = tx_out.send(Message::Binary(ack.encode()));
    }
}

// ─── public entry ───────────────────────────────────────────────────────────

/// 启动飞书长连接（后台任务，自动重连）。失败绝不导致 Server 退出。
pub fn start_feishu_bot(
    app_id: String,
    app_secret: String,
    handler: Arc<dyn FeishuBotHandler>,
) -> FeishuBotHandle {
    let cancel = CancellationToken::new();
    let bot_open_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let app_id_c = app_id.clone();
    let app_secret_c = app_secret.clone();
    let handler_c = handler.clone();
    let bot_open_id_c = bot_open_id.clone();
    let cancel_c = cancel.clone();

    let task = tokio::spawn(async move {
        // best-effort fetch bot open_id for @mention detection
        match api::get_bot_info(&app_id_c, &app_secret_c).await {
            Ok((open_id, _)) => {
                if let Ok(mut g) = bot_open_id_c.lock() {
                    *g = Some(open_id);
                }
            }
            Err(e) => logger::error("feishu bot info fetch failed (mention detection degraded)", Some(&e)),
        }

        while !cancel_c.is_cancelled() {
            handler_c.on_state(false);
            let cfg = match fetch_endpoint(&app_id_c, &app_secret_c).await {
                Ok(c) => c,
                Err(e) => {
                    logger::error("feishu endpoint fetch failed", Some(&e));
                    if !sleep_cancellable(&cancel_c, 30_000).await {
                        break;
                    }
                    continue;
                }
            };
            let result = run_connection(
                &cfg,
                &app_id_c,
                &app_secret_c,
                &handler_c,
                &bot_open_id_c,
                &cancel_c,
            )
            .await;
            handler_c.on_state(false);
            if let Err(e) = result {
                logger::error("feishu long connection dropped", Some(&e));
            }
            let delay = cfg.reconnect_interval_ms.max(3000) + (cfg.reconnect_nonce_ms % 1000);
            if !sleep_cancellable(&cancel_c, delay).await {
                break;
            }
        }
    });

    FeishuBotHandle { cancel, task }
}

async fn sleep_cancellable(cancel: &CancellationToken, ms: u64) -> bool {
    tokio::select! {
        _ = cancel.cancelled() => false,
        _ = tokio::time::sleep(std::time::Duration::from_millis(ms)) => true,
    }
}
