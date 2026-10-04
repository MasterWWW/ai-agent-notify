import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { DEFAULT_HOST, DEFAULT_PORT } from "./version.js";

export interface ServerConfig {
  host: string;
  port: number;
  token: string;
  /** Where the token was resolved from. */
  tokenSource: "cli" | "env" | "file" | "generated";
}

export const STATE_DIR = join(homedir(), ".ai-task-notify");
export const TOKEN_FILE = join(STATE_DIR, "token");
export const APP_CONFIG_FILE = join(STATE_DIR, "config.json");
export const EVENTS_FILE = join(STATE_DIR, "events.jsonl");

export interface AppConfig {
  /** 自定义机器人 Webhook（备选） */
  feishuWebhook?: string;
  feishuSecret?: string;
  /** 自建应用机器人（推荐） */
  feishuAppId?: string;
  feishuAppSecret?: string;
  feishuChatId?: string;
  /** 用户 open_id（发到与机器人的单聊） */
  feishuOpenId?: string;
  /** 手机号（仅用于 App 里查询 open_id，方便回填） */
  feishuMobile?: string;
}

export function loadAppConfig(): AppConfig {
  try {
    if (!existsSync(APP_CONFIG_FILE)) return {};
    return JSON.parse(readFileSync(APP_CONFIG_FILE, "utf8")) as AppConfig;
  } catch {
    return {};
  }
}

export function saveAppConfig(cfg: AppConfig): void {
  ensureStateDir();
  writeFileSync(APP_CONFIG_FILE, JSON.stringify(cfg, null, 2), { mode: 0o600 });
}

function ensureStateDir(): void {
  mkdirSync(STATE_DIR, { recursive: true });
}

function readTokenFile(): string | null {
  try {
    if (!existsSync(TOKEN_FILE)) return null;
    const t = readFileSync(TOKEN_FILE, "utf8").trim();
    return t || null;
  } catch {
    return null;
  }
}

function writeTokenFile(token: string): void {
  try {
    ensureStateDir();
    writeFileSync(TOKEN_FILE, token, { mode: 0o600 });
  } catch {
    // Best effort; token is still printed on the console.
  }
}

function generateToken(): string {
  return randomBytes(24).toString("hex");
}

export function resolveToken(cliToken?: string): { token: string; source: ServerConfig["tokenSource"] } {
  if (cliToken) return { token: cliToken, source: "cli" };
  const env = process.env.AI_TASK_NOTIFY_TOKEN;
  if (env) return { token: env, source: "env" };
  const file = readTokenFile();
  if (file) return { token: file, source: "file" };
  const generated = generateToken();
  writeTokenFile(generated);
  return { token: generated, source: "generated" };
}

export function readTokenForHook(cliToken?: string): string | null {
  if (cliToken) return cliToken;
  if (process.env.AI_TASK_NOTIFY_TOKEN) return process.env.AI_TASK_NOTIFY_TOKEN;
  return readTokenFile();
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
