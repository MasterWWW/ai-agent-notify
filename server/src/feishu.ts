import { createHmac, randomUUID } from "node:crypto";
import type { AgentEvent } from "./types.js";
import { error } from "./logger.js";

export interface FeishuConfig {
  webhook: string;
  /** Optional signing secret (custom bot security setting). */
  secret?: string;
}

const STATUS_ICON: Record<string, string> = {
  success: "✅",
  waiting: "🟡",
  info: "💬",
};

function buildText(event: AgentEvent): string {
  const agentLabel = event.agent === "claude" ? "Claude Code" : "Codex";
  const statusIcon = STATUS_ICON[event.status] ?? "";
  const lines: string[] = [];
  lines.push(`🤖 ${agentLabel}`);
  if (event.project) lines.push(event.project);
  lines.push(`${statusIcon} ${event.title}`);
  if (event.message) lines.push(event.message);
  return lines.join("\n");
}

/** Feishu custom-bot signature: HMAC-SHA256 with key = `${timestamp}\n${secret}`, empty message. */
function sign(timestamp: string, secret: string): string {
  const stringToSign = `${timestamp}\n${secret}`;
  return createHmac("sha256", stringToSign).digest("base64");
}

/**
 * Send an event to a Feishu custom bot webhook. Fire-and-forget:
 * failures are logged (without secrets) and never block the agent.
 */
export async function sendFeishu(cfg: FeishuConfig, event: AgentEvent): Promise<void> {
  const body: Record<string, unknown> = {
    msg_type: "text",
    content: { text: buildText(event) },
  };
  if (cfg.secret) {
    const timestamp = String(Math.floor(Date.now() / 1000));
    body.timestamp = timestamp;
    body.sign = sign(timestamp, cfg.secret);
  }
  try {
    const res = await fetch(cfg.webhook, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      error(`feishu send failed: HTTP ${res.status}`);
      return;
    }
    const data = (await res.json().catch(() => ({}))) as { code?: number; msg?: string };
    if (data.code !== undefined && data.code !== 0) {
      error(`feishu send failed: code=${data.code} msg=${data.msg ?? ""}`);
    }
  } catch (err) {
    error("feishu send failed (ignored)", err);
  }
}

export { randomUUID };
