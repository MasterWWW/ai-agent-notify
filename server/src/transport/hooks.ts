import { basename } from "node:path";
import type { Agent, AgentEvent, EventStatus, EventType } from "../domain/types.js";

export type HookKind =
  | "codex-stop"
  | "codex-permission"
  | "codex-subagent-stop"
  | "claude-stop"
  | "claude-permission"
  | "claude-notification";

interface HookInput {
  hook_event_name?: string;
  cwd?: string;
  project?: string;
  [key: string]: unknown;
}

function projectOf(input: HookInput): string | undefined {
  if (typeof input.project === "string" && input.project) return input.project;
  if (typeof input.cwd === "string" && input.cwd) return basename(input.cwd) || undefined;
  return undefined;
}

function firstString(...vals: unknown[]): string | undefined {
  for (const v of vals) {
    if (typeof v === "string" && v.trim()) return v.trim();
  }
  return undefined;
}

function summarize(value: unknown, max = 200): string | undefined {
  if (typeof value === "string") {
    const s = value.trim();
    if (!s) return undefined;
    return s.length > max ? `${s.slice(0, max)}…` : s;
  }
  if (value && typeof value === "object") {
    try {
      const s = JSON.stringify(value);
      return s.length > max ? `${s.slice(0, max)}…` : s;
    } catch {
      return undefined;
    }
  }
  return undefined;
}

function extractCodexPermissionMessage(input: HookInput): string | undefined {
  const perms = input.permissions;
  if (Array.isArray(perms) && perms.length > 0) {
    const parts: string[] = [];
    for (const p of perms.slice(0, 3)) {
      const obj = p as Record<string, unknown>;
      const part =
        firstString(obj.command, obj.question, obj.explanation, obj.kind) ??
        summarize(obj);
      if (part) parts.push(part);
    }
    if (parts.length > 0) return parts.join(" | ").slice(0, 300);
  }
  return (
    firstString(input.reason, input.question, input.explanation) ??
    summarize(input.payload)
  );
}

function extractClaudePermissionMessage(input: HookInput): string | undefined {
  const pr = input.permission_request as Record<string, unknown> | undefined;
  if (pr && typeof pr === "object") {
    return firstString(pr.question, pr.permission, pr.action, pr.message) ?? summarize(pr);
  }
  return firstString(input.question, input.message, input.action) ?? summarize(input.payload);
}

function extractClaudeNotificationMessage(input: HookInput): { title?: string; message?: string } {
  const m = input.message;
  if (typeof m === "string" && m.trim()) {
    const s = m.trim();
    const title = s.length > 60 ? `${s.slice(0, 60)}…` : s;
    return { title: "Claude Code 通知", message: title };
  }
  if (input.title) return { title: firstString(input.title), message: undefined };
  return { title: "Claude Code 通知", message: undefined };
}

/** Map a raw hook stdin payload to a unified AgentEvent (document §7/§8). */
export function mapHookToEvent(kind: HookKind, input: HookInput): AgentEvent {
  const base: { agent: Agent; project?: string; cwd?: string; timestamp: number; metadata?: Record<string, unknown> } = {
    agent: kind.startsWith("codex") ? "codex" : "claude",
    timestamp: Date.now(),
  };
  const cwd = typeof input.cwd === "string" ? input.cwd : undefined;
  if (cwd) base.cwd = cwd;
  const project = projectOf(input);
  if (project) base.project = project;

  switch (kind) {
    case "codex-stop": {
      const ev: AgentEvent = {
        ...base,
        id: "",
        event: "task_completed",
        status: "success",
        title: "Codex 任务完成",
        message: firstString(input.summary, input.result) ?? undefined,
      };
      return ev;
    }
    case "codex-permission": {
      const ev: AgentEvent = {
        ...base,
        id: "",
        event: "permission_required",
        status: "waiting",
        title: "Codex 等待权限",
        message: extractCodexPermissionMessage(input),
      };
      return ev;
    }
    case "codex-subagent-stop": {
      const ev: AgentEvent = {
        ...base,
        id: "",
        event: "subagent_completed",
        status: "success",
        title: "Codex 子任务完成",
        message: firstString(input.summary, input.result) ?? undefined,
      };
      return ev;
    }
    case "claude-stop": {
      const ev: AgentEvent = {
        ...base,
        id: "",
        event: "task_completed",
        status: "success",
        title: "Claude Code 任务完成",
        message: firstString(input.summary, input.result) ?? undefined,
      };
      return ev;
    }
    case "claude-permission": {
      const ev: AgentEvent = {
        ...base,
        id: "",
        event: "permission_required",
        status: "waiting",
        title: "Claude Code 等待权限",
        message: extractClaudePermissionMessage(input),
      };
      return ev;
    }
    case "claude-notification": {
      const { title, message } = extractClaudeNotificationMessage(input);
      const ev: AgentEvent = {
        ...base,
        id: "",
        event: "notification",
        status: "info",
        title: title ?? "Claude Code 通知",
        message,
      };
      return ev;
    }
  }
}

export type { AgentEvent, EventStatus, EventType };
