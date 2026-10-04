export type Agent = "codex" | "claude";

export type EventType =
  | "task_completed"
  | "permission_required"
  | "notification"
  | "subagent_completed";

export type EventStatus = "success" | "waiting" | "info";

/** Unified event model (document §6). */
export interface AgentEvent {
  id: string;
  agent: Agent;
  event: EventType;
  status: EventStatus;
  project?: string;
  cwd?: string;
  branch?: string;
  title: string;
  message?: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

/** Event as accepted from CLI/hooks (id is assigned by the server). */
export type EventInput = Omit<AgentEvent, "id" | "timestamp"> & { timestamp?: number };

export interface HealthResponse {
  status: "ok";
  version: string;
}

export interface IngestResponse {
  success: true;
  eventId: string;
}

export type ServerMessage =
  | { type: "connected"; serverVersion: string }
  | { type: "agent_event"; payload: AgentEvent }
  | { type: "pong" };
