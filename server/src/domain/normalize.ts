import { randomBytes } from "node:crypto";
import { log } from "../core/logger.js";
import type { Agent, AgentEvent, EventStatus, EventType } from "./types.js";

const AGENTS = new Set<Agent>(["codex", "claude"]);
const EVENTS = new Set<EventType>([
  "task_completed",
  "permission_required",
  "notification",
  "subagent_completed",
]);
const STATUSES = new Set<EventStatus>(["success", "waiting", "info"]);

function newEventId(): string {
  return `evt_${randomBytes(8).toString("hex")}`;
}

export interface IncomingEvent {
  agent?: unknown;
  event?: unknown;
  status?: unknown;
  project?: unknown;
  cwd?: unknown;
  branch?: unknown;
  title?: unknown;
  message?: unknown;
  timestamp?: unknown;
  metadata?: unknown;
}

/**
 * Validate + normalize an incoming HTTP event into a full AgentEvent.
 * Throws on invalid input.
 */
export function normalizeEvent(input: IncomingEvent): AgentEvent {
  const agent = input.agent;
  const event = input.event;
  const status = input.status ?? defaultStatus(event as string);

  if (typeof agent !== "string" || !AGENTS.has(agent as Agent)) {
    throw new Error(`invalid agent: ${String(agent)}`);
  }
  if (typeof event !== "string" || !EVENTS.has(event as EventType)) {
    throw new Error(`invalid event: ${String(event)}`);
  }
  if (typeof status !== "string" || !STATUSES.has(status as EventStatus)) {
    throw new Error(`invalid status: ${String(status)}`);
  }
  const title = input.title;
  if (typeof title !== "string" || title.trim() === "") {
    throw new Error("title is required");
  }

  const timestamp =
    typeof input.timestamp === "number" && Number.isFinite(input.timestamp)
      ? input.timestamp
      : Date.now();

  const eventObj: AgentEvent = {
    id: newEventId(),
    agent: agent as Agent,
    event: event as EventType,
    status: status as EventStatus,
    title: title.trim(),
    timestamp,
  };
  if (typeof input.project === "string" && input.project) eventObj.project = input.project;
  if (typeof input.cwd === "string" && input.cwd) eventObj.cwd = input.cwd;
  if (typeof input.branch === "string" && input.branch) eventObj.branch = input.branch;
  if (typeof input.message === "string" && input.message) eventObj.message = input.message;
  if (input.metadata && typeof input.metadata === "object") {
    eventObj.metadata = input.metadata as Record<string, unknown>;
  }
  return eventObj;
}

function defaultStatus(event: string): EventStatus | undefined {
  if (event === "permission_required") return "waiting";
  if (event === "notification") return "info";
  if (event === "task_completed" || event === "subagent_completed") return "success";
  return undefined;
}

/** Structured log line for one event (used by the LogHandler). */
export function logEvent(event: AgentEvent, clientCount: number): void {
  log({
    msg: "event",
    eventId: event.id,
    agent: event.agent,
    event: event.event,
    status: event.status,
    project: event.project,
    clients: clientCount,
  });
}
