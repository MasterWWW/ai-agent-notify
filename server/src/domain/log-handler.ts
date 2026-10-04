import type { WsHub } from "../transport/ws.js";
import { logEvent } from "./normalize.js";
import type { AgentEvent } from "./types.js";
import type { EventHandler } from "./pipeline.js";

/** Logs each event (with current WS client count). */
export class LogHandler implements EventHandler {
  readonly name = "log";

  constructor(private readonly hub: WsHub) {}

  handle(event: AgentEvent): void {
    logEvent(event, this.hub.clientCount);
  }
}
