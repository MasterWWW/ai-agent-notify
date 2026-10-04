import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

export const STATE_DIR = join(homedir(), ".ai-task-notify");
export const TOKEN_FILE = join(STATE_DIR, "token");
export const APP_CONFIG_FILE = join(STATE_DIR, "config.json");
export const EVENTS_FILE = join(STATE_DIR, "events.jsonl");

export type TokenSource = "cli" | "env" | "file" | "generated";

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

/** Resolve the server token with precedence: --token > env > file > generated. */
export function resolveToken(cliToken?: string): { token: string; source: TokenSource } {
  if (cliToken) return { token: cliToken, source: "cli" };
  const env = process.env.AI_TASK_NOTIFY_TOKEN;
  if (env) return { token: env, source: "env" };
  const file = readTokenFile();
  if (file) return { token: file, source: "file" };
  const generated = generateToken();
  writeTokenFile(generated);
  return { token: generated, source: "generated" };
}

/** Read the token for hook/CLI clients (does not generate). */
export function readTokenForHook(cliToken?: string): string | null {
  if (cliToken) return cliToken;
  if (process.env.AI_TASK_NOTIFY_TOKEN) return process.env.AI_TASK_NOTIFY_TOKEN;
  return readTokenFile();
}
