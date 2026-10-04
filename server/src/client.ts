import type { EventInput } from "./types.js";

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
