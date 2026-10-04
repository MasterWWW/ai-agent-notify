import { createHmac } from "node:crypto";
import type { AgentEvent } from "../../domain/types.js";
import { buildText } from "./render.js";
import { getTenantToken, resetTokenCache, FEISHU_BASE } from "./api.js";

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

async function sendAppMessage(cfg: FeishuConfig, text: string): Promise<void> {
  const receiveIdType = cfg.openId ? "open_id" : "chat_id";
  const receiveId = cfg.openId ?? cfg.chatId ?? "";
  const send = async (): Promise<void> => {
    const token = await getTenantToken(cfg.appId!, cfg.appSecret!);
    const res = await fetch(`${FEISHU_BASE}/im/v1/messages?receive_id_type=${receiveIdType}`, {
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
      resetTokenCache();
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
 * Send one event to Feishu. Prefers the self-built app bot, falls back to the
 * custom Webhook. Throws on failure; the caller decides how to record it
 * (notification failures must never block the Agent).
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

/** Send a test message to verify the config. Returns a human-readable result. */
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
