import { readFileSync } from "node:fs";
import { VERSION } from "./version.js";
import { loadAppConfig, parseServerArgs, readTokenForHook, saveAppConfig, type AppConfig } from "./config.js";
import { startApp } from "./app.js";
import { error, log } from "./logger.js";
import { getHealth, postEvent, resolveBaseUrl, type ClientOptions } from "./client.js";
import { mapHookToEvent, type HookKind } from "./hooks.js";
import { listChats, testFeishu } from "./feishu.js";
import type { EventInput } from "./types.js";

const HOOK_KINDS: HookKind[] = [
  "codex-stop",
  "codex-permission",
  "codex-subagent-stop",
  "claude-stop",
  "claude-permission",
  "claude-notification",
];

function printUsage(): void {
  console.log(`AI Task Notify v${VERSION}

Usage:
  ai-task-notify server [--host 0.0.0.0] [--port 3210] [--token xxx]
  ai-task-notify status [--base-url http://127.0.0.1:3210]
  ai-task-notify test [--message "..."] [--project "..."] [--token xxx] [--base-url ...]
  ai-task-notify config --feishu-app-id <id> --feishu-app-secret <secret> --feishu-chat-id <chatId>
  ai-task-notify config --feishu-webhook <url> [--feishu-secret <secret>]
  ai-task-notify config --show
  ai-task-notify config --clear-feishu
  ai-task-notify feishu test          # 发送测试消息，验证飞书配置
  ai-task-notify feishu chats         # 列出机器人所在的群（需要 im:chat:readonly 权限）
  ai-task-notify hook <kind> [--token xxx] [--base-url ...]

Hook kinds:
  ${HOOK_KINDS.join("\n  ")}

Env:
  AI_TASK_NOTIFY_TOKEN       token (used when --token is absent)
  AI_TASK_NOTIFY_BASE_URL    server base url for hooks/test/status
`);
}

function mask(s: string | undefined): string {
  if (!s) return "(未设置)";
  return s.length > 12 ? s.slice(0, 4) + "…" + s.slice(-4) : "***";
}

async function cmdConfig(argv: string[]): Promise<number> {
  const webhook = argValue(argv, "--feishu-webhook");
  const secret = argValue(argv, "--feishu-secret");
  const appId = argValue(argv, "--feishu-app-id");
  const appSecret = argValue(argv, "--feishu-app-secret");
  const chatId = argValue(argv, "--feishu-chat-id");
  if (argv.includes("--show")) {
    const cfg = loadAppConfig();
    console.log(`feishuMode=${cfg.feishuAppId ? "自建应用机器人" : cfg.feishuWebhook ? "Webhook 机器人" : "未配置"}`);
    console.log(`feishuAppId=${cfg.feishuAppId ?? "(未设置)"}`);
    console.log(`feishuAppSecret=${mask(cfg.feishuAppSecret)}`);
    console.log(`feishuChatId=${cfg.feishuChatId ?? "(未设置)"}`);
    console.log(`feishuWebhook=${cfg.feishuWebhook ?? "(未设置)"}`);
    console.log(`feishuSecret=${mask(cfg.feishuSecret)}`);
    return 0;
  }
  if (argv.includes("--clear-feishu")) {
    const cfg = loadAppConfig();
    const keys: (keyof AppConfig)[] = ["feishuWebhook", "feishuSecret", "feishuAppId", "feishuAppSecret", "feishuChatId"];
    for (const k of keys) delete cfg[k];
    saveAppConfig(cfg);
    console.log("feishu config cleared");
    return 0;
  }
  if (appId || appSecret || chatId) {
    const cfg = loadAppConfig();
    if (appId) cfg.feishuAppId = appId;
    if (appSecret) cfg.feishuAppSecret = appSecret;
    if (chatId) cfg.feishuChatId = chatId;
    saveAppConfig(cfg);
    console.log("feishu app bot config saved");
    return 0;
  }
  if (webhook) {
    const cfg = loadAppConfig();
    cfg.feishuWebhook = webhook;
    if (secret) cfg.feishuSecret = secret;
    else if (argv.includes("--feishu-secret")) cfg.feishuSecret = "";
    saveAppConfig(cfg);
    console.log("feishu webhook saved");
    return 0;
  }
  console.error("usage: ai-task-notify config --feishu-app-id <id> --feishu-app-secret <secret> --feishu-chat-id <chatId> | --feishu-webhook <url> [--feishu-secret <secret>] | --show | --clear-feishu");
  return 1;
}

async function cmdFeishu(argv: string[]): Promise<number> {
  const sub = argv[0];
  const cfg = loadAppConfig();
  if (sub === "test") {
    console.log(await testFeishu({
      webhook: cfg.feishuWebhook,
      webhookSecret: cfg.feishuSecret,
      appId: cfg.feishuAppId,
      appSecret: cfg.feishuAppSecret,
      chatId: cfg.feishuChatId,
    }));
    return 0;
  }
  if (sub === "chats") {
    if (!cfg.feishuAppId || !cfg.feishuAppSecret) {
      console.error("需要先配置 --feishu-app-id 和 --feishu-app-secret");
      return 1;
    }
    try {
      const chats = await listChats(cfg.feishuAppId, cfg.feishuAppSecret);
      if (chats.length === 0) {
        console.log("（没有找到机器人所在的群）");
        return 0;
      }
      for (const c of chats) console.log(`${c.chat_id}	${c.name}`);
      return 0;
    } catch (err) {
      console.error(`获取群列表失败：${err instanceof Error ? err.message : String(err)}`);
      return 1;
    }
  }
  console.error("usage: ai-task-notify feishu test | feishu chats");
  return 1;
}

async function cmdServer(argv: string[]): Promise<number> {
  const cfg = parseServerArgs(argv);
  const app = await startApp({ host: cfg.host, port: cfg.port, token: cfg.token });
  log({ msg: "token (put this in the phone app)", token: cfg.token });
  log({
    msg: "phone should connect to",
    url: `ws://${app.hostname}:${app.port}/ws?token=${cfg.token}`,
  });
  log({ msg: "token source", source: cfg.tokenSource });

  const shutdown = () => {
    log({ msg: "shutting down" });
    // Exit promptly even if the http/ws close hangs.
    app.close().finally(() => process.exit(0));
    setTimeout(() => process.exit(0), 1500).unref();
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
  return 0;
}

async function cmdStatus(argv: string[]): Promise<number> {
  const baseUrl = resolveBaseUrl(argValue(argv, "--base-url"));
  try {
    const health = await getHealth(baseUrl);
    console.log(`status=${health.status} version=${health.version ?? "?"} base=${baseUrl}`);
    return 0;
  } catch (err) {
    error("server not reachable", err);
    return 1;
  }
}

async function cmdTest(argv: string[]): Promise<number> {
  const baseUrl = resolveBaseUrl(argValue(argv, "--base-url"));
  const token = readTokenForHook(argValue(argv, "--token"));
  if (!token) {
    console.error("ai-task-notify: no token. Set AI_TASK_NOTIFY_TOKEN or start the server once to generate ~/.ai-task-notify/token");
    return 1;
  }
  const opts: ClientOptions = { baseUrl, token };
  const event: EventInput = {
    agent: "codex",
    event: "task_completed",
    status: "success",
    title: "测试通知",
    message: argValue(argv, "--message") ?? "AI Task Notify 链路测试",
    project: argValue(argv, "--project") ?? "ai-task-notify",
  };
  try {
    const eventId = await postEvent(opts, event);
    console.log(`sent eventId=${eventId}`);
    return 0;
  } catch (err) {
    error("send test event failed", err);
    return 1;
  }
}

async function cmdHook(argv: string[]): Promise<number> {
  const kind = argv[0] as HookKind | undefined;
  if (!kind || !HOOK_KINDS.includes(kind)) {
    console.error(`ai-task-notify: unknown hook kind '${kind ?? ""}'`);
    return 1;
  }
  const baseUrl = resolveBaseUrl(argValue(argv.slice(1), "--base-url"));
  const token = readTokenForHook(argValue(argv.slice(1), "--token"));
  const opts: ClientOptions = { baseUrl, token: token ?? "" };

  try {
    const raw = await readStdin();
    let input: Record<string, unknown> = {};
    if (raw.trim()) {
      input = JSON.parse(raw) as Record<string, unknown>;
    }
    const event = mapHookToEvent(kind, input);
    const { id: _id, ...body } = event;
    // Keep hooks silent on success (the server already logs the event).
    await postEvent(opts, body);
  } catch (err) {
    // Hook failures must NEVER break the agent (document §26).
    error("hook forwarding failed (ignored)", err);
  }
  return 0;
}

function argValue(argv: string[], name: string): string | undefined {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
}

function readStdin(): Promise<string> {
  return new Promise((resolvePromise) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (c) => (data += c));
    process.stdin.on("end", () => resolvePromise(data));
    process.stdin.on("error", () => resolvePromise(data));
  });
}

export async function main(argv: string[]): Promise<number> {
  // Tolerate a leading "--" (e.g. `pnpm start -- server`).
  if (argv[0] === "--") argv = argv.slice(1);
  const [cmd, ...rest] = argv;
  switch (cmd) {
    case "server":
      return cmdServer(rest);
    case "status":
      return cmdStatus(rest);
    case "test":
      return cmdTest(rest);
    case "config":
      return cmdConfig(rest);
    case "feishu":
      return cmdFeishu(rest);
    case "hook":
      return cmdHook(rest);
    case "version":
      console.log(VERSION);
      return 0;
    case "--help":
    case "-h":
    case undefined:
      printUsage();
      return 0;
    default:
      console.error(`ai-task-notify: unknown command '${cmd}'`);
      printUsage();
      return 1;
  }
}

// Allow direct execution: node dist/index.js server ...
function isDirectRun(): boolean {
  try {
    if (typeof require !== "undefined" && require.main === module) return true;
  } catch {
    // CJS require is unavailable under ESM; fall through.
  }
  if (!process.argv[1]) return false;
  try {
    return import.meta.url.endsWith(process.argv[1]);
  } catch {
    return false;
  }
}

if (isDirectRun()) {
  main(process.argv.slice(2)).then((code) => {
    process.exitCode = code;
  });
}
