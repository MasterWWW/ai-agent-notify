import { startApp } from "../transport/server.js";
import { log } from "../core/logger.js";
import { resolveToken, type TokenSource } from "../core/state.js";
import { DEFAULT_HOST, DEFAULT_PORT } from "../core/version.js";
import { LogHandler } from "../domain/log-handler.js";
import { HistoryHandler } from "../features/history.js";
import { NotifierHandler } from "../features/notifier.js";
import { loadAppConfig } from "../features/config.js";
import { argValue } from "./util.js";

export interface ServerConfig {
  host: string;
  port: number;
  token: string;
  tokenSource: TokenSource;
}

export function parseServerArgs(argv: string[]): ServerConfig {
  let host = DEFAULT_HOST;
  let port = DEFAULT_PORT;
  let token: string | undefined;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--host") host = argv[++i] ?? host;
    else if (a === "--port") port = Number(argv[++i]) || port;
    else if (a === "--token") token = argv[++i];
    else if (a === "--help" || a === "-h") {
      console.log(
        "Usage: ai-task-notify server [--host 0.0.0.0] [--port 3210] [--token xxx]\n" +
        "Token precedence: --token > AI_TASK_NOTIFY_TOKEN > ~/.ai-task-notify/token (auto-generated)"
      );
      process.exit(0);
    }
  }
  const { token: resolved, source } = resolveToken(token);
  return { host, port, token: resolved, tokenSource: source };
}

/** Start the server with the default handler wiring (composition root). */
export async function cmdServer(argv: string[]): Promise<number> {
  const cfg = parseServerArgs(argv);
  const app = await startApp(
    { host: cfg.host, port: cfg.port, token: cfg.token },
    (hub) => [
      new LogHandler(hub),
      new HistoryHandler(),
      new NotifierHandler(loadAppConfig),
    ]
  );
  log({ msg: "token (put this in the phone app)", token: cfg.token });
  log({
    msg: "phone should connect to",
    url: `ws://${app.hostname}:${app.port}/ws?token=${cfg.token}`,
  });
  log({ msg: "token source", source: cfg.tokenSource });

  const shutdown = () => {
    log({ msg: "shutting down" });
    // Exit promptly even if the http/ws close hangs.
    app.close().finally(() => process.exit(0));
    setTimeout(() => process.exit(0), 1500).unref();
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
  return 0;
}
