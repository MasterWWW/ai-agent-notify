import { error } from "../core/logger.js";
import type { AgentEvent } from "./types.js";

/**
 * A consumer of AgentEvents. Handlers are independent: each one owns a single
 * concern (broadcast, history, notify...) and never knows about the others.
 */
export interface EventHandler {
  readonly name: string;
  handle(event: AgentEvent): void | Promise<void>;
}

/**
 * Business orchestration: one event in, fan out to every registered handler.
 * A failing handler never breaks the pipeline or the caller (and thus never
 * breaks an Agent). Handlers are decoupled from both transport and each other.
 */
export class Pipeline {
  private readonly handlers: EventHandler[] = [];

  register(handler: EventHandler): void {
    this.handlers.push(handler);
  }

  async handle(event: AgentEvent): Promise<void> {
    for (const h of this.handlers) {
      try {
        await h.handle(event);
      } catch (err) {
        error(`pipeline handler "${h.name}" failed (ignored)`, err);
      }
    }
  }
}
