use ai_task_notify_server::client::{
    get_health, post_event, post_permission_request, resolve_base_url, wait_permission_decision, ClientOptions,
};
use ai_task_notify_server::config::{load_app_config, save_app_config};
use ai_task_notify_server::feishu::channels::{feishu_config_from, test_feishu};
use ai_task_notify_server::feishu::queries::{list_chats, resolve_open_id_by_mobile};
use ai_task_notify_server::hooks::{extract_permission_input, is_safe_command, map_hook_to_event, HookKind};
use ai_task_notify_server::model::{Agent, DEFAULT_HOST, DEFAULT_PORT, VERSION};
use ai_task_notify_server::state::{read_token_for_hook, resolve_token};
use ai_task_notify_server::{build_state, logger, start_feishu_bot_component};
use serde_json::Value;

fn print_usage() {
    println!("AI Task Notify v{VERSION}");
    println!();
    println!("Usage:");
    println!("  ai-task-notify server [--host 0.0.0.0] [--port 3210] [--token xxx]");
    println!("  ai-task-notify status [--base-url http://127.0.0.1:3210]");
    println!("  ai-task-notify test [--message \"...\"] [--project \"...\"] [--token xxx] [--base-url ...]");
    println!("  ai-task-notify config --feishu-app-id <id> --feishu-app-secret <secret> [--feishu-chat-id <chatId>]");
    println!("  ai-task-notify feishu me --mobile <手机号>   # 查你的 open_id，直接发到与机器人的单聊");
    println!("  ai-task-notify config --feishu-webhook <url> [--feishu-secret <secret>]");
    println!("  ai-task-notify config --show");
    println!("  ai-task-notify config --clear-feishu");
    println!("  ai-task-notify feishu test          # 发送测试消息，验证飞书配置");
    println!("  ai-task-notify feishu chats         # 列出机器人所在的群/会话（需要 im:chat:readonly 权限）");
    println!("  ai-task-notify hook <kind> [--token xxx] [--base-url ...]");
    println!();
    println!("Hook kinds:");
    for k in HookKind::all() {
        println!("  {}", k.as_str());
    }
    println!();
    println!("Env:");
    println!("  AI_TASK_NOTIFY_TOKEN       token (used when --token is absent)");
    println!("  AI_TASK_NOTIFY_BASE_URL    server base url for hooks/test/status");
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn args_has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn mask(s: Option<&str>) -> String {
    match s {
        None | Some("") => "(未设置)".to_string(),
        Some(s) if s.chars().count() > 12 => {
            let c: Vec<char> = s.chars().collect();
            let head: String = c[..4].iter().collect();
            let tail: String = c[c.len() - 4..].iter().collect();
            format!("{head}…{tail}")
        }
        Some(_) => "***".to_string(),
    }
}

async fn read_stdin() -> String {
    use tokio::io::AsyncReadExt;
    let mut buf = String::new();
    let _ = tokio::io::stdin().read_to_string(&mut buf).await;
    buf
}

// ─── server ────────────────────────────────────────────────────────────────

async fn cmd_server(args: &[String]) -> i32 {
    let mut host = DEFAULT_HOST.to_string();
    let mut port = DEFAULT_PORT;
    let mut cli_token: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                i += 1;
                if let Some(h) = args.get(i) {
                    host = h.clone();
                }
            }
            "--port" => {
                i += 1;
                if let Some(p) = args.get(i).and_then(|s| s.parse().ok()) {
                    port = p;
                }
            }
            "--token" => {
                i += 1;
                cli_token = args.get(i).cloned();
            }
            "--help" | "-h" => {
                println!("Usage: ai-task-notify server [--host 0.0.0.0] [--port 3210] [--token xxx]");
                println!("Token precedence: --token > AI_TASK_NOTIFY_TOKEN > ~/.ai-task-notify/token (auto-generated)");
                return 0;
            }
            _ => {}
        }
        i += 1;
    }

    let (token, source) = resolve_token(cli_token);
    let state = build_state(token.clone());
    let app = match ai_task_notify_server::http::serve(state.clone(), &host, port, true).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ai-task-notify: {e}");
            return 1;
        }
    };

    logger::log(&[("msg", Some("token (put this in the phone app)")), ("token", Some(&token))]);
    logger::log(&[
        ("msg", Some("phone should connect to")),
        ("url", Some(&format!("ws://{}:{}/ws?token={}", app.hostname, app.port, token))),
    ]);
    logger::log(&[("msg", Some("token source")), ("source", Some(source.as_str()))]);

    let bot = start_feishu_bot_component(&state);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            logger::log(&[("msg", Some("shutting down"))]);
        }
        _ = shutdown_signal() => {
            logger::log(&[("msg", Some("shutting down"))]);
        }
    }

    app.close().await;
    if let Some(b) = bot {
        b.stop().await;
    }
    0
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            let _ = sigterm.recv().await;
        }
    }
    #[cfg(not(unix))]
    {
        std::future::pending::<()>().await;
    }
}

// ─── status ────────────────────────────────────────────────────────────────

async fn cmd_status(args: &[String]) -> i32 {
    let base_url = resolve_base_url(arg_value(args, "--base-url"));
    match get_health(&base_url).await {
        Ok((status, version)) => {
            println!("status={status} version={version} base={base_url}");
            0
        }
        Err(e) => {
            eprintln!("ai-task-notify: server not reachable: {e}");
            1
        }
    }
}

// ─── test ──────────────────────────────────────────────────────────────────

async fn cmd_test(args: &[String]) -> i32 {
    let base_url = resolve_base_url(arg_value(args, "--base-url"));
    let Some(token) = read_token_for_hook(arg_value(args, "--token")) else {
        eprintln!("ai-task-notify: no token. Set AI_TASK_NOTIFY_TOKEN or start the server once to generate ~/.ai-task-notify/token");
        return 1;
    };
    let opts = ClientOptions { base_url, token };
    let input = ai_task_notify_server::model::EventInput {
        agent: Agent::Codex,
        event: ai_task_notify_server::model::EventType::TaskCompleted,
        status: Some(ai_task_notify_server::model::EventStatus::Success),
        title: "测试通知".to_string(),
        message: Some(arg_value(args, "--message").unwrap_or_else(|| "AI Task Notify 链路测试".to_string())),
        project: Some(arg_value(args, "--project").unwrap_or_else(|| "ai-task-notify".to_string())),
        cwd: None,
        branch: None,
        timestamp: None,
        metadata: None,
    };
    match post_event(&opts, &input).await {
        Ok(event_id) => {
            println!("sent eventId={event_id}");
            0
        }
        Err(e) => {
            eprintln!("ai-task-notify: send test event failed: {e}");
            1
        }
    }
}

// ─── config ────────────────────────────────────────────────────────────────

fn cmd_config(args: &[String]) -> i32 {
    if args_has(args, "--show") {
        let cfg = load_app_config();
        println!(
            "feishuMode={}",
            if cfg.feishu_app_id.is_some() {
                "自建应用机器人"
            } else if cfg.feishu_webhook.is_some() {
                "Webhook 机器人"
            } else {
                "未配置"
            }
        );
        println!("feishuAppId={}", cfg.feishu_app_id.as_deref().unwrap_or("(未设置)"));
        println!("feishuAppSecret={}", mask(cfg.feishu_app_secret.as_deref()));
        println!("feishuChatId={}", cfg.feishu_chat_id.as_deref().unwrap_or("(未设置)"));
        println!("feishuOpenId={}", cfg.feishu_open_id.as_deref().unwrap_or("(未设置)"));
        println!("feishuMobile={}", cfg.feishu_mobile.as_deref().unwrap_or("(未设置)"));
        println!("feishuWebhook={}", cfg.feishu_webhook.as_deref().unwrap_or("(未设置)"));
        println!("feishuSecret={}", mask(cfg.feishu_secret.as_deref()));
        return 0;
    }
    if args_has(args, "--clear-feishu") {
        let mut cfg = load_app_config();
        cfg.feishu_webhook = None;
        cfg.feishu_secret = None;
        cfg.feishu_app_id = None;
        cfg.feishu_app_secret = None;
        cfg.feishu_chat_id = None;
        cfg.feishu_open_id = None;
        cfg.feishu_mobile = None;
        let _ = save_app_config(&cfg);
        println!("feishu config cleared");
        return 0;
    }
    let app_id = arg_value(args, "--feishu-app-id");
    let app_secret = arg_value(args, "--feishu-app-secret");
    let chat_id = arg_value(args, "--feishu-chat-id");
    if app_id.is_some() || app_secret.is_some() || chat_id.is_some() {
        let mut cfg = load_app_config();
        if let Some(v) = app_id {
            cfg.feishu_app_id = Some(v);
        }
        if let Some(v) = app_secret {
            cfg.feishu_app_secret = Some(v);
        }
        if let Some(v) = chat_id {
            cfg.feishu_chat_id = Some(v);
        }
        let _ = save_app_config(&cfg);
        println!("feishu app bot config saved");
        return 0;
    }
    let open_id = arg_value(args, "--feishu-open-id");
    let mobile = arg_value(args, "--feishu-mobile");
    if open_id.is_some() || mobile.is_some() {
        let mut cfg = load_app_config();
        if let Some(v) = open_id {
            cfg.feishu_open_id = Some(v);
        }
        if let Some(v) = mobile {
            cfg.feishu_mobile = Some(v);
        }
        let _ = save_app_config(&cfg);
        println!("feishu open_id saved");
        return 0;
    }
    if let Some(webhook) = arg_value(args, "--feishu-webhook") {
        let mut cfg = load_app_config();
        cfg.feishu_webhook = Some(webhook);
        if let Some(secret) = arg_value(args, "--feishu-secret") {
            cfg.feishu_secret = Some(secret);
        } else if args_has(args, "--feishu-secret") {
            cfg.feishu_secret = Some(String::new());
        }
        let _ = save_app_config(&cfg);
        println!("feishu webhook saved");
        return 0;
    }
    eprintln!("usage: ai-task-notify config --feishu-app-id <id> --feishu-app-secret <secret> --feishu-chat-id <chatId> | --feishu-webhook <url> [--feishu-secret <secret>] | --show | --clear-feishu");
    1
}

// ─── feishu ────────────────────────────────────────────────────────────────

async fn cmd_feishu(args: &[String]) -> i32 {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("");
    let cfg = load_app_config();
    match sub {
        "me" => {
            let Some(mobile) = arg_value(args, "--mobile") else {
                eprintln!("usage: ai-task-notify feishu me --mobile <手机号>");
                return 1;
            };
            let (Some(app_id), Some(app_secret)) = (&cfg.feishu_app_id, &cfg.feishu_app_secret) else {
                eprintln!("需要先配置 --feishu-app-id 和 --feishu-app-secret");
                return 1;
            };
            match resolve_open_id_by_mobile(app_id, app_secret, &mobile).await {
                Ok(open_id) => {
                    let mut cfg = load_app_config();
                    cfg.feishu_open_id = Some(open_id.clone());
                    cfg.feishu_mobile = Some(mobile);
                    let _ = save_app_config(&cfg);
                    println!("✅ 已找到你的 open_id={open_id}");
                    println!("已保存：之后消息会直接发到你和机器人的单聊窗口。");
                    0
                }
                Err(e) => {
                    eprintln!("查询失败：{e}");
                    1
                }
            }
        }
        "test" => {
            let result = test_feishu(&feishu_config_from(&cfg)).await;
            println!("{result}");
            0
        }
        "chats" => {
            let (Some(app_id), Some(app_secret)) = (&cfg.feishu_app_id, &cfg.feishu_app_secret) else {
                eprintln!("需要先配置 --feishu-app-id 和 --feishu-app-secret");
                return 1;
            };
            match list_chats(app_id, app_secret).await {
                Ok(chats) => {
                    if chats.is_empty() {
                        println!("（没有找到机器人所在的群）");
                    } else {
                        for (chat_id, name) in chats {
                            println!("{chat_id}\t{name}");
                        }
                    }
                    0
                }
                Err(e) => {
                    eprintln!("获取群列表失败：{e}");
                    1
                }
            }
        }
        "connect" => {
            let (Some(app_id), Some(app_secret)) = (&cfg.feishu_app_id, &cfg.feishu_app_secret) else {
                eprintln!("需要先配置 --feishu-app-id 和 --feishu-app-secret");
                return 1;
            };
            println!("正在连接飞书长连接（WebSocket），Ctrl+C 退出……");
            let state = build_state(String::new());
            let bot = start_feishu_bot_component(&state);
            let bot = match bot {
                Some(b) => b,
                None => {
                    eprintln!("连接失败：缺少 App ID/Secret");
                    return 1;
                }
            };
            let _ = app_id;
            let _ = app_secret;
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = shutdown_signal() => {}
            }
            bot.stop().await;
            0
        }
        _ => {
            eprintln!("usage: ai-task-notify feishu test | feishu chats | feishu me --mobile <手机号> | feishu connect");
            1
        }
    }
}

// ─── hook ──────────────────────────────────────────────────────────────────

fn print_decision(behavior: &str) {
    let decision = if behavior == "allow" {
        serde_json::json!({ "behavior": "allow" })
    } else {
        serde_json::json!({ "behavior": "deny", "message": "你在飞书拒绝了该操作" })
    };
    let out = serde_json::json!({
        "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision }
    });
    println!("{out}");
}

async fn cmd_hook_permission_wait(kind: HookKind, opts: &ClientOptions) -> i32 {
    let raw = read_stdin().await;
    let input: Value = if raw.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&raw).unwrap_or(Value::Null)
    };
    let agent = kind.agent();
    let body = extract_permission_input(agent, &input, &std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    // 安全命令自动放行（不发卡片），默认开启，可用 AI_TASK_NOTIFY_AUTO_ALLOW=0 关闭。
    let auto_allow = std::env::var("AI_TASK_NOTIFY_AUTO_ALLOW").map(|v| v != "0").unwrap_or(true);
    if auto_allow && body.tool_name == "Bash" {
        if let Some(cmd) = &body.command {
            if is_safe_command(cmd) {
                print_decision("allow");
                return 0;
            }
        }
    }
    let result = async {
        let request_id = post_permission_request(opts, &body).await?;
        let (status, decision) = wait_permission_decision(opts, &request_id, 540_000).await?;
        Ok::<_, String>((status, decision))
    }
    .await;
    match result {
        Ok((status, decision)) => match (status.as_str(), decision.as_deref()) {
            ("allowed", _) => print_decision("allow"),
            ("denied", _) => print_decision("deny"),
            _ => {} // timeout / 其他状态：不输出决定 → 终端审批流兜底
        },
        Err(_) => {
            // 完全静默：失败=无决定输出=Agent 回退终端审批流
        }
    }
    0
}

async fn cmd_hook(args: &[String]) -> i32 {
    let Some(kind_str) = args.first() else {
        eprintln!("ai-task-notify: unknown hook kind ''");
        return 1;
    };
    let Some(kind) = HookKind::parse(kind_str) else {
        eprintln!("ai-task-notify: unknown hook kind '{kind_str}'");
        return 1;
    };
    let rest = &args[1..];
    let base_url = resolve_base_url(arg_value(rest, "--base-url"));
    let token = read_token_for_hook(arg_value(rest, "--token")).unwrap_or_default();
    let opts = ClientOptions { base_url, token };

    if kind.is_permission() && args_has(rest, "--wait") {
        return cmd_hook_permission_wait(kind, &opts).await;
    }

    let raw = read_stdin().await;
    let input: Value = if raw.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&raw).unwrap_or(Value::Null)
    };
    let event = map_hook_to_event(kind, &input);
    match post_event(&opts, &event).await {
        Ok(_) => {}
        Err(e) => {
            // Hook failures must NEVER break the agent
            logger::error("hook forwarding failed (ignored)", Some(&e));
        }
    }
    0
}

// ─── main ──────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let argv = if argv.first().map(|s| s.as_str()) == Some("--") {
        argv[1..].to_vec()
    } else {
        argv
    };
    let (cmd, rest) = match argv.split_first() {
        Some((c, r)) => (c.as_str(), r.to_vec()),
        None => {
            print_usage();
            return;
        }
    };
    let code = match cmd {
        "server" => cmd_server(&rest).await,
        "status" => cmd_status(&rest).await,
        "test" => cmd_test(&rest).await,
        "config" => cmd_config(&rest),
        "feishu" => cmd_feishu(&rest).await,
        "hook" => cmd_hook(&rest).await,
        "version" => {
            println!("{VERSION}");
            0
        }
        "--help" | "-h" => {
            print_usage();
            0
        }
        other => {
            eprintln!("ai-task-notify: unknown command '{other}'");
            print_usage();
            1
        }
    };
    std::process::exit(code);
}
