import type { AgentEvent } from "../../domain/types.js";

const STATUS_ICON: Record<string, string> = {
  success: "✅",
  waiting: "🟡",
  info: "💬",
};

/** Render an AgentEvent into the human-readable notification text. */
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
