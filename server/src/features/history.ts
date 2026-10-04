import { appendFileSync, existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { EVENTS_FILE } from "../core/state.js";
import type { AgentEvent } from "../domain/types.js";
import type { EventHandler } from "../domain/pipeline.js";

/** Persists events to ~/.ai-task-notify/events.jsonl (for the macOS app UI), capping size. */
export class HistoryHandler implements EventHandler {
  readonly name = "history";

  handle(event: AgentEvent): void {
    try {
      const line = JSON.stringify(event) + "\n";
      if (existsSync(EVENTS_FILE) && statSync(EVENTS_FILE).size > 1_000_000) {
        const lines = readFileSync(EVENTS_FILE, "utf8").split("\n").filter(Boolean).slice(-100);
        writeFileSync(EVENTS_FILE, lines.join("\n") + "\n");
      }
      appendFileSync(EVENTS_FILE, line);
    } catch {
      // non-critical
    }
  }
}
