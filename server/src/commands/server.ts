import { startApp } from "../transport/server.js";
import { log, error } from "../core/logger.js";
import { resolveToken, type TokenSource } from "../core/state.js";
import { DEFAULT_HOST, DEFAULT_PORT } from "../core/version.js";
import { LogHandler } from "../domain/log-handler.js";
import { HistoryHandler } from "../features/history.js";
import { NotifierHandler } from "../features/notifier.js";
import { loadAppConfig } from "../features/config.js";
import { startFeishuBot, type FeishuBot } from "../features/feishu/connection.js";
import { handleBotMessage, type BotInfo } from "../features/feishu/bot.js";
import { PermissionStore } from "../domain/permission.js";
import { PermissionService } from "../features/permission.js";
import { argValue } from "./util.js";

export interface ServerConfig {
  host: string;
  port: number;
  token: string;
  tokenSource: TokenSource;
}

export function parseServerArgs(argv: string[]): ServerConfig {
  let host = DEFAULT_HOST;
  let port = DEFAULT_PORT;
  let token: string | undefined;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--host") host = argv[++i] ?? host;
    else if (a === "--port") port = Number(argv[++i]) || port;
    else if (a === "--token") token = argv[++i];
    else if (a === "--help" || a === "-h") {
      console.log(
        "Usage: ai-task-notify server [--host 0.0.0.0] [--port 3210] [--token xxx]\n" +
        "Token precedence: --token > AI_TASK_NOTIFY_TOKEN > ~/.ai-task-notify/token (auto-generated)"
      );
      process.exit(0);
    }
  }
  const { token: resolved, source } = resolveToken(token);
  return { host, port, token: resolved, tokenSource: source };
}

/** Start the server with the default handler wiring (composition root). */
export async function cmdServer(argv: string[]): Promise<number> {
  const cfg = parseServerArgs(argv);
  // 权限请求交互（飞书卡片远程控制）：Server 内建内存存储 + HTTP API。
  const permissionStore = new PermissionStore();
  const permissionService = new PermissionService(permissionStore);
  const app = await startApp(
    { host: cfg.host, port: cfg.port, token: cfg.token },
    (hub) => [
      new LogHandler(hub),
      new HistoryHandler(),
      new NotifierHandler(loadAppConfig),
    ],
    permissionService
  );
  log({ msg: "token (put this in the phone app)", token: cfg.token });
  log({
    msg: "phone should connect to",
    url: `ws://${app.hostname}:${app.port}/ws?token=${cfg.token}`,
  });
  log({ msg: "token source", source: cfg.tokenSource });

  // 接管飞书自建应用机器人的聊天（长连接）：只依赖 App ID / Secret。
  // 失败只记录日志，不影响 Server 与通知功能。
  let feishuBot: FeishuBot | null = null;
  const appCfg = loadAppConfig();
  if (appCfg.feishuAppId && appCfg.feishuAppSecret) {
    const botInfo: BotInfo = { connected: false };
    startFeishuBot(appCfg.feishuAppId, appCfg.feishuAppSecret, {
      onMessage: (msg) =>
        handleBotMessage(msg, {
          botInfo: () => botInfo,
          onTextDecision: (rid, act, senderOpenId) => permissionService.resolveFromText(rid, act, senderOpenId),
        }),
      onCardAction: (evt) => permissionService.handleCardAction(evt),
      onState: (state) => {
        botInfo.connected = state === "connected";
        log({ msg: "feishu bot state", state });
      },
      onReady: () => {
        botInfo.connected = true;
        log({ msg: "feishu bot ready (长连接已建立，私聊机器人即可发送命令)" });
      },
    })
      .then((bot) => {
        feishuBot = bot;
      })
      .catch((err) => {
        botInfo.connected = false;
        error("feishu bot connect failed (server keeps running)", err);
      });
  } else {
    log({ msg: "feishu bot skipped (未配置 App ID/Secret)" });
  }

  const shutdown = () => {
    log({ msg: "shutting down" });
    // Exit promptly even if the http/ws close hangs.
    const tasks: Promise<unknown>[] = [app.close().catch(() => {})];
    permissionStore.stop();
    if (feishuBot) tasks.push(feishuBot.stop().catch(() => {}));
    Promise.allSettled(tasks).finally(() => process.exit(0));
    setTimeout(() => process.exit(0), 1500).unref();
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
  return 0;
}
