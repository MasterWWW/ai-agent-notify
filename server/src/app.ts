import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { WebSocketServer } from "ws";
import { VERSION } from "./version.js";
import { bearerToken, isValidToken } from "./auth.js";
import { Hub } from "./hub.js";
import { log, error } from "./logger.js";
import { logEvent, normalizeEvent, type IncomingEvent } from "./events.js";
import { loadAppConfig, EVENTS_FILE } from "./config.js";
import { sendFeishu, logFeishuError } from "./feishu.js";
import { getLocalHostname, getLanIps, publishBonjourService, stopBonjour } from "./mdns.js";
import type { AgentEvent } from "./types.js";
import { appendFileSync, statSync, readFileSync, writeFileSync, existsSync } from "node:fs";

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

/** Append an event to ~/.ai-task-notify/events.jsonl (for the macOS app UI), capping size. */
function appendEventFile(event: AgentEvent): void {
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

export function startApp(opts: AppOptions): Promise<RunningApp> {
  return new Promise((resolve, reject) => {
    const hubHolder: { hub?: Hub } = {};

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
            const event: AgentEvent = normalizeEvent(parsed);
            const clients = hubHolder.hub?.broadcast(event) ?? 0;
            logEvent(event, clients);
            appendEventFile(event);
            const appCfg = loadAppConfig();
            if (appCfg.feishuAppId || appCfg.feishuWebhook) {
              void sendFeishu(
                {
                  webhook: appCfg.feishuWebhook,
                  webhookSecret: appCfg.feishuSecret,
                  appId: appCfg.feishuAppId,
                  appSecret: appCfg.feishuAppSecret,
                  chatId: appCfg.feishuChatId,
                },
                event
              ).catch(logFeishuError);
            }
            json(res, 200, { success: true, eventId: event.id });
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

    const hub = new Hub(wss, opts.token);
    hubHolder.hub = hub;

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
