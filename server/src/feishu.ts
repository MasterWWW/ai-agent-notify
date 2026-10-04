import { createHmac } from "node:crypto";
import type { AgentEvent } from "./types.js";
import { error } from "./logger.js";

export interface FeishuConfig {
  /** 自定义机器人 Webhook（备选方案） */
  webhook?: string;
  webhookSecret?: string;
  /** 自建应用机器人（推荐） */
  appId?: string;
  appSecret?: string;
  /** 群聊 Chat ID（receive_id_type=chat_id） */
  chatId?: string;
  /** 用户 open_id（receive_id_type=open_id，直接发到与机器人的单聊） */
  openId?: string;
}

const BASE = "https://open.feishu.cn/open-apis";
const STATUS_ICON: Record<string, string> = {
  success: "✅",
  waiting: "🟡",
  info: "💬",
};

export function buildText(event: AgentEvent): string {
  const agentLabel = event.agent === "claude" ? "Claude Code" : "Codex";
  const statusIcon = STATUS_ICON[event.status] ?? "";
  const lines: string[] = [];
  lines.push(`🤖 ${agentLabel}`);
  if (event.project) lines.push(event.project);
  lines.push(`${statusIcon} ${event.title}`);
  if (event.message) lines.push(event.message);
  return lines.join("\n");
}

/** 自定义机器人 Webhook 签名：HMAC-SHA256，key = `${timestamp}\n${secret}`，空消息。 */
function webhookSign(timestamp: string, secret: string): string {
  return createHmac("sha256", `${timestamp}\n${secret}`).digest("base64");
}

async function sendWebhook(cfg: FeishuConfig, text: string): Promise<void> {
  const body: Record<string, unknown> = { msg_type: "text", content: { text } };
  if (cfg.webhookSecret) {
    const timestamp = String(Math.floor(Date.now() / 1000));
    body.timestamp = timestamp;
    body.sign = webhookSign(timestamp, cfg.webhookSecret);
  }
  const res = await fetch(cfg.webhook!, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = (await res.json().catch(() => ({}))) as { code?: number; msg?: string };
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  if (data.code !== undefined && data.code !== 0) throw new Error(`code=${data.code} msg=${data.msg ?? ""}`);
}

// ---- 自建应用机器人 ----

let tokenCache: { token: string; expiresAt: number } | null = null;

async function getTenantToken(appId: string, appSecret: string): Promise<string> {
  if (tokenCache && Date.now() < tokenCache.expiresAt - 60_000) return tokenCache.token;
  const res = await fetch(`${BASE}/auth/v3/tenant_access_token/internal`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ app_id: appId, app_secret: appSecret }),
  });
  const data = (await res.json().catch(() => ({}))) as {
    code?: number;
    msg?: string;
    tenant_access_token?: string;
    expire?: number;
  };
  if (data.code !== 0 || !data.tenant_access_token) {
    throw new Error(`获取 tenant_access_token 失败: code=${data.code} msg=${data.msg ?? ""}`);
  }
  tokenCache = {
    token: data.tenant_access_token,
    expiresAt: Date.now() + (data.expire ?? 7200) * 1000,
  };
  return tokenCache.token;
}

async function sendAppMessage(cfg: FeishuConfig, text: string): Promise<void> {
  const receiveIdType = cfg.openId ? "open_id" : "chat_id";
  const receiveId = cfg.openId ?? cfg.chatId ?? "";
  const send = async (): Promise<void> => {
    const token = await getTenantToken(cfg.appId!, cfg.appSecret!);
    const res = await fetch(`${BASE}/im/v1/messages?receive_id_type=${receiveIdType}`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
      body: JSON.stringify({
        receive_id: receiveId,
        msg_type: "text",
        content: JSON.stringify({ text }),
      }),
    });
    const data = (await res.json().catch(() => ({}))) as { code?: number; msg?: string };
    if (data.code === 99991663 || data.code === 99991664 || res.status === 401) {
      // token 过期/无效，清缓存重试一次
      tokenCache = null;
      throw new Error(`TOKEN_EXPIRED:${data.code}`);
    }
    if (data.code !== 0) throw new Error(`code=${data.code} msg=${data.msg ?? ""}`);
  };
  try {
    await send();
  } catch (err) {
    if (err instanceof Error && err.message.startsWith("TOKEN_EXPIRED")) {
      await send();
      return;
    }
    throw err;
  }
}

/**
 * 通过手机号查用户的 open_id（用于发到与机器人的单聊）。
 * 需要 contact:user.base:readonly 权限。
 */
export async function resolveOpenIdByMobile(
  appId: string,
  appSecret: string,
  mobile: string
): Promise<string> {
  const token = await getTenantToken(appId, appSecret);
  const res = await fetch(`${BASE}/contact/v3/users/batch_get_id?user_id_type=open_id`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({ mobiles: [mobile] }),
  });
  const data = (await res.json().catch(() => ({}))) as {
    code?: number;
    msg?: string;
    data?: { user_list?: Array<{ user_id?: string; mobile?: string }> };
  };
  if (data.code !== 0) throw new Error(`code=${data.code} msg=${data.msg ?? ""}`);
  const user = data.data?.user_list?.[0];
  if (!user?.user_id) throw new Error("没有找到该手机号对应的用户（请确认手机号在组织通讯录内）");
  return user.user_id;
}

/** 列出机器人所在的群/会话（用于让用户挑选 chat_id）。需要 im:chat:readonly 权限。 */
export async function listChats(appId: string, appSecret: string): Promise<Array<{ chat_id: string; name: string }>> {
  const token = await getTenantToken(appId, appSecret);
  const res = await fetch(`${BASE}/im/v1/chats?page_size=50`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  const data = (await res.json().catch(() => ({}))) as { code?: number; msg?: string; data?: { items?: Array<{ chat_id: string; name?: string }> } };
  if (data.code !== 0) throw new Error(`code=${data.code} msg=${data.msg ?? ""}`);
  return (data.data?.items ?? []).map((c) => ({ chat_id: c.chat_id, name: c.name ?? "(未命名群)" }));
}

/**
 * 根据配置发送事件到飞书。优先自建应用机器人，其次自定义 Webhook。
 * 失败抛出异常，由调用方决定是否记录（绝不阻断 Agent）。
 */
export async function sendFeishu(cfg: FeishuConfig, event: AgentEvent): Promise<void> {
  const text = buildText(event);
  if (cfg.appId && cfg.appSecret && (cfg.chatId || cfg.openId)) {
    await sendAppMessage(cfg, text);
    return;
  }
  if (cfg.webhook) {
    await sendWebhook(cfg, text);
    return;
  }
  throw new Error("飞书未配置（需要 App ID/Secret/Chat ID 或 Webhook）");
}

/** 发送一条测试消息，用于验证配置。返回人类可读的结果。 */
export async function testFeishu(cfg: FeishuConfig): Promise<string> {
  const text = "✅ AI Task Notify 飞书配置测试成功\n链路正常，可以开始接收任务通知了。";
  try {
    if (cfg.appId && cfg.appSecret && (cfg.chatId || cfg.openId)) {
      await sendAppMessage(cfg, text);
      return cfg.openId ? "✅ 发送成功（自建应用机器人 · 单聊）" : "✅ 发送成功（自建应用机器人 · 群聊）";
    }
    if (cfg.webhook) {
      await sendWebhook(cfg, text);
      return "✅ 发送成功（Webhook 机器人）";
    }
    return "❌ 未配置飞书（需要 App ID/Secret + open_id 或 Chat ID，或 Webhook）";
  } catch (err) {
    return `❌ 发送失败：${err instanceof Error ? err.message : String(err)}`;
  }
}

export function logFeishuError(err: unknown): void {
  error("feishu send failed (ignored)", err);
}
