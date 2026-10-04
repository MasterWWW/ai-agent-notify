import { WebSocket, WebSocketServer } from "ws";
import type { IncomingMessage } from "node:http";
import { log } from "../core/logger.js";
import { isValidToken } from "../core/auth.js";
import type { AgentEvent, ServerMessage } from "../domain/types.js";

/** WebSocket hub: manages clients and broadcasts AgentEvents. No business logic. */
export class WsHub {
  private clients = new Set<WebSocket>();

  constructor(private wss: WebSocketServer, private token: string) {
    wss.on("connection", (ws: WebSocket, req: IncomingMessage) => {
      const url = new URL(req.url ?? "/", "http://localhost");
      const token = url.searchParams.get("token");
      if (!isValidToken(token, this.token)) {
        ws.close(4001, "unauthorized");
        return;
      }
      this.clients.add(ws);
      log({ msg: "ws client connected", clients: this.clients.size });
      this.send(ws, { type: "connected", serverVersion: process.env.AI_TASK_NOTIFY_VERSION ?? "0.1.0" });
      ws.on("message", (data) => this.onMessage(ws, data));
      ws.on("close", () => {
        this.clients.delete(ws);
        log({ msg: "ws client disconnected", clients: this.clients.size });
      });
      ws.on("error", () => {
        this.clients.delete(ws);
      });
    });
  }

  private onMessage(ws: WebSocket, data: WebSocket.RawData): void {
    try {
      const parsed = JSON.parse(data.toString());
      if (parsed?.type === "ping") {
        this.send(ws, { type: "pong" });
      }
      // Other message types are ignored in phase 1.
    } catch {
      // Ignore malformed messages.
    }
  }

  private send(ws: WebSocket, msg: ServerMessage): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    }
  }

  /** Broadcast an event to every connected client. */
  broadcast(event: AgentEvent): number {
    const msg: ServerMessage = { type: "agent_event", payload: event };
    let ok = 0;
    for (const ws of this.clients) {
      try {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify(msg));
          ok++;
        }
      } catch {
        // Drop failed clients silently; they will reconnect.
      }
    }
    return ok;
  }

  get clientCount(): number {
    return this.clients.size;
  }
}
