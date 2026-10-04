import type { NormalizedMessage } from "@larksuiteoapi/node-sdk";
import { log } from "../../core/logger.js";
import { DEFAULT_PORT, VERSION } from "../../core/version.js";
import { getLocalHostname } from "../../transport/mdns.js";
import type { AgentEvent } from "../../domain/types.js";
import { loadAppConfig, saveAppConfig } from "../config.js";
import { readRecentEvents } from "../history.js";

/** 机器人连接状态（供 status 命令展示）。 */
export interface BotInfo {
  connected: boolean;
  botName?: string;
}

/** 文本兜底决定回调：返回给用户的提示文本。 */
export type TextDecisionHandler = (rid: string, act: "allow" | "deny", senderOpenId: string) => Promise<string> | string;

const ICON: Record<AgentEvent["status"], string> = { success: "✅", waiting: "🟡", info: "💬" };

const HELP = `🤖 AI Task Notify 机器人

可用命令：
ping        测试连通
help        查看帮助
status      查看服务与机器人状态
current     查看当前/最近任务
recent      查看最近 10 条事件

直接发消息即可，不需要 @。`;

/** 清洗消息文本：去掉 @提及 / <at> 标签。 */
export function cleanText(raw: string): string {
  return raw
    .replace(/<at[^>]*>.*?<\/at>/g, " ")
    .replace(/@_user_\d+/g, " ")
    .replace(/@\S+/g, " ")
    .trim();
}

/** 清洗消息文本：去掉 @提及 / <at> 标签，取第一个词作为命令。 */
export function parseCommand(raw: string): string {
  const cleaned = cleanText(raw).toLowerCase();
  return cleaned.split(/\s+/)[0] ?? "";
}

function fmtTime(ts: number): string {
  return new Date(ts).toLocaleTimeString("zh-CN", { hour12: false });
}

function formatEventLine(e: AgentEvent): string {
  const icon = ICON[e.status] ?? "•";
  const project = e.project ? ` ${e.project}` : "";
  const title = e.title ? ` ${e.title}` : "";
  return `${icon} [${fmtTime(e.timestamp)}] ${e.agent}${project}${title}`;
}

function buildStatus(bot: BotInfo): string {
  const cfg = loadAppConfig();
  const last = readRecentEvents(1)[0];
  const lines = [
    "🤖 AI Task Notify",
    "",
    `服务：运行中 (v${VERSION})`,
    `主机：${getLocalHostname()}:${DEFAULT_PORT}`,
    `机器人：${bot.connected ? "已连接 ✅" : "未连接 ❌"}`,
  ];
  if (cfg.feishuChatId || cfg.feishuOpenId) lines.push("目标：已绑定你的单聊 ✅");
  else lines.push("目标：未绑定（先给机器人发一条消息即可自动绑定）");
  lines.push(last
    ? `最近事件：${fmtTime(last.timestamp)} ${last.agent} ${last.event} ${last.status}`
    : "最近事件：暂无");
  return lines.join("\n");
}

function buildCurrent(): string {
  const events = readRecentEvents(20);
  const waiting = events.find((e) => e.status === "waiting");
  const target = waiting ?? events[events.length - 1];
  if (!target) return "当前没有运行中的任务。";
  return `${waiting ? "⏳ 当前等待确认" : "📌 最近任务"}\n\n${formatEventLine(target)}`;
}

function buildRecent(): string {
  const events = readRecentEvents(10);
  if (events.length === 0) return "还没有事件记录。";
  return `📋 最近 ${events.length} 条事件\n\n${events.map(formatEventLine).join("\n")}`;
}

/**
 * 处理一条发给机器人的消息，返回要回复的文本；返回 null 表示不回复。
 * 私聊消息会自动绑定发送者为通知目标（保存 open_id + chat_id）。
 */
export async function handleBotMessage(
  msg: NormalizedMessage,
  deps: { botInfo: () => BotInfo; onTextDecision?: TextDecisionHandler }
): Promise<string | null> {
  // 只接管私聊；群聊必须 @机器人
  if (msg.chatType !== "p2p" && !msg.mentionedBot) return null;

  // 自动绑定：第一次收到私聊就记住 open_id 和 chat_id，之后通知直接进这个单聊
  if (msg.chatType === "p2p" && msg.senderId) {
    try {
      const cfg = loadAppConfig();
      if (cfg.feishuAppId) {
        let changed = false;
        if (!cfg.feishuOpenId && msg.senderId) { cfg.feishuOpenId = msg.senderId; changed = true; }
        if (!cfg.feishuChatId && msg.chatId) { cfg.feishuChatId = msg.chatId; changed = true; }
        if (changed) {
          saveAppConfig(cfg);
          log({ msg: "feishu sender auto-bound", chatId: msg.chatId, openId: msg.senderId.slice(0, 6) + "…" });
        }
      }
    } catch {
      // 绑定失败不影响回复
    }
  }

  // 文本兜底：允许 <id> / 拒绝 <id>（卡片回调异常时可用）
  if (deps.onTextDecision) {
    const m = /^(允许|拒绝)\s+(pr_[A-Za-z0-9]+)$/.exec(cleanText(msg.content));
    if (m) {
      const act = m[1] === "允许" ? "allow" : "deny";
      return deps.onTextDecision(m[2], act, msg.senderId);
    }
  }

  const cmd = parseCommand(msg.content);
  switch (cmd) {
    case "ping":
      return "pong";
    case "help":
      return HELP;
    case "status":
      return buildStatus(deps.botInfo());
    case "current":
      return buildCurrent();
    case "recent":
      return buildRecent();
    default:
      return `未识别的命令：${cmd || "(空)"}\n\n发送 help 查看可用命令。`;
  }
}
