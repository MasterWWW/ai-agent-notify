import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { WebSocketServer } from "ws";
import { VERSION } from "../core/version.js";
import { bearerToken, isValidToken } from "../core/auth.js";
import { WsHub } from "./ws.js";
import { error, log } from "../core/logger.js";
import { normalizeEvent, type IncomingEvent } from "../domain/normalize.js";
import { Pipeline, type EventHandler } from "../domain/pipeline.js";
import { getLocalHostname, getLanIps, publishBonjourService, stopBonjour } from "./mdns.js";

export interface AppOptions {
  host: string;
  port: number;
  token: string;
  /** Set to false to skip publishing mDNS (useful in tests). */
  mdns?: boolean;
}

export interface RunningApp {
  port: number;
  hostname: string;
  lanIps: string[];
  close: () => Promise<void>;
}

/**
 * Register the event handlers for the app. The server only knows about the
 * transport; handlers are supplied by the composition root (commands/server).
 */
export type HandlerFactory = (hub: WsHub) => EventHandler[];

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    req.on("data", (c: Buffer) => chunks.push(c));
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });
}

function json(res: ServerResponse, code: number, body: unknown): void {
  const payload = JSON.stringify(body);
  res.writeHead(code, { "Content-Type": "application/json; charset=utf-8" });
  res.end(payload);
}

/**
 * Start the HTTP + WebSocket server. Transport only: authenticate, normalize,
 * hand the event to the Pipeline, respond. No business logic lives here.
 */
export function startApp(opts: AppOptions, handlers: HandlerFactory): Promise<RunningApp> {
  return new Promise((resolve, reject) => {
    const server = createServer((req, res) => {
      const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
      const path = url.pathname;

      if (req.method === "GET" && path === "/health") {
        json(res, 200, { status: "ok", version: VERSION });
        return;
      }

      if (req.method === "POST" && path === "/api/events") {
        const token = bearerToken(req.headers.authorization);
        if (!isValidToken(token, opts.token)) {
          json(res, 401, { success: false, error: "unauthorized" });
          return;
        }
        readBody(req)
          .then((body) => {
            let parsed: IncomingEvent;
            try {
              parsed = JSON.parse(body);
            } catch {
              json(res, 400, { success: false, error: "invalid json" });
              return;
            }
            const event = normalizeEvent(parsed);
            pipeline.handle(event).then(() => {
              json(res, 200, { success: true, eventId: event.id });
            });
          })
          .catch((err) => {
            error("event rejected", err);
            json(res, 400, { success: false, error: err instanceof Error ? err.message : "invalid event" });
          });
        return;
      }

      json(res, 404, { success: false, error: "not found" });
    });

    const wss = new WebSocketServer({ noServer: true });
    server.on("upgrade", (req, socket, head) => {
      const url = new URL(req.url ?? "/", "http://localhost");
      if (url.pathname !== "/ws") {
        socket.destroy();
        return;
      }
      const token = url.searchParams.get("token");
      if (!isValidToken(token, opts.token)) {
        socket.write("HTTP/1.1 401 Unauthorized\r\n\r\n");
        socket.destroy();
        return;
      }
      wss.handleUpgrade(req, socket, head, (ws) => {
        wss.emit("connection", ws, req);
      });
    });

    const hub = new WsHub(wss, opts.token);
    const pipeline = new Pipeline();
    for (const h of handlers(hub)) pipeline.register(h);

    server.on("error", reject);

    server.listen(opts.port, opts.host, () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : opts.port;
      const hostname = getLocalHostname();
      const lanIps = getLanIps();

      log({ msg: "server started", version: VERSION, port, bind: opts.host });
      log({ msg: "lan hostname (use this on the phone)", hostname: `${hostname}:${port}` });
      if (lanIps.length > 0) {
        log({ msg: "lan ips (for first-time check only)", ips: lanIps.join(",") });
      } else {
        log({ msg: "no private LAN ip detected; is the Mac connected to the LAN?" });
      }
      log({ msg: "token source", source: opts.token === "?" ? "?" : "resolved" });

      if (opts.mdns !== false) {
        publishBonjourService("AI Task Notify", "ai-task-notify", port);
      }

      resolve({
        port,
        hostname,
        lanIps,
        close: () => {
          stopBonjour();
          return new Promise<void>((r) => {
            for (const ws of hub["clients"] as Set<unknown>) {
              try {
                (ws as { close?: () => void }).close?.();
              } catch {
                // ignore
              }
            }
            wss.close();
            server.close(() => r());
          });
        },
      });
    });
  });
}
