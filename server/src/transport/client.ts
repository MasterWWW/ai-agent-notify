import type { EventInput } from "../domain/types.js";

export interface ClientOptions {
  baseUrl: string;
  token: string;
}

export function resolveBaseUrl(cliBaseUrl?: string): string {
  if (cliBaseUrl) return cliBaseUrl.replace(/\/+$/, "");
  const env = process.env.AI_TASK_NOTIFY_BASE_URL;
  if (env) return env.replace(/\/+$/, "");
  return "http://127.0.0.1:3210";
}

export async function getHealth(baseUrl: string): Promise<{ status: string; version?: string }> {
  const res = await fetch(`${baseUrl}/health`);
  if (!res.ok) throw new Error(`health check failed: HTTP ${res.status}`);
  return (await res.json()) as { status: string; version?: string };
}

export async function postEvent(opts: ClientOptions, event: EventInput): Promise<string> {
  const res = await fetch(`${opts.baseUrl}/api/events`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${opts.token}`,
    },
    body: JSON.stringify(event),
  });
  const body = (await res.json().catch(() => ({}))) as { success?: boolean; eventId?: string; error?: string };
  if (!res.ok || body.success !== true) {
    throw new Error(body.error ?? `HTTP ${res.status}`);
  }
  return body.eventId ?? "unknown";
}

// ─── 权限请求交互（飞书卡片远程控制） ───────────────────────────────

export interface PermissionCreateInput {
  agent: string;
  toolName: string;
  command?: string;
  cwd?: string;
}

export interface PermissionWaitResult {
  status: string;
  decision?: "allow" | "deny";
}

/** 提交一个权限请求给本地 Server（Server 会发飞书卡片）。 */
export async function postPermissionRequest(
  opts: ClientOptions,
  input: PermissionCreateInput
): Promise<{ requestId: string }> {
  const res = await fetch(`${opts.baseUrl}/api/permission-requests`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${opts.token}`,
    },
    body: JSON.stringify(input),
  });
  const body = (await res.json().catch(() => ({}))) as { success?: boolean; requestId?: string; error?: string };
  if (!res.ok || body.success !== true || !body.requestId) {
    throw new Error(body.error ?? `HTTP ${res.status}`);
  }
  return { requestId: body.requestId };
}

/** 长轮询等待权限请求决定；超时返回 { status: "timeout" }。 */
export async function waitPermissionDecision(
  opts: ClientOptions,
  requestId: string,
  timeoutMs: number
): Promise<PermissionWaitResult> {
  const res = await fetch(
    `${opts.baseUrl}/api/permission-requests/${encodeURIComponent(requestId)}/wait?timeout=${Math.trunc(timeoutMs)}`,
    {
      headers: { Authorization: `Bearer ${opts.token}` },
    }
  );
  const body = (await res.json().catch(() => ({}))) as { success?: boolean; status?: string; decision?: "allow" | "deny"; error?: string };
  if (!res.ok || body.success !== true || !body.status) {
    throw new Error(body.error ?? `HTTP ${res.status}`);
  }
  return { status: body.status, decision: body.decision };
}
