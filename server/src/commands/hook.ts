import { error } from "../core/logger.js";
import { readTokenForHook } from "../core/state.js";
import { mapHookToEvent, type HookKind } from "../transport/hooks.js";
import { postEvent, resolveBaseUrl, type ClientOptions } from "../transport/client.js";

export const HOOK_KINDS: HookKind[] = [
  "codex-stop",
  "codex-permission",
  "codex-subagent-stop",
  "claude-stop",
  "claude-permission",
  "claude-notification",
];

function readStdin(): Promise<string> {
  return new Promise((resolvePromise) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (c) => (data += c));
    process.stdin.on("end", () => resolvePromise(data));
    process.stdin.on("error", () => resolvePromise(data));
  });
}

/**
 * Forward a Hook payload to the local server as an AgentEvent.
 * Hooks must stay silent on success and NEVER break the agent on failure.
 */
export async function cmdHook(argv: string[]): Promise<number> {
  const kind = argv[0] as HookKind | undefined;
  if (!kind || !HOOK_KINDS.includes(kind)) {
    console.error(`ai-task-notify: unknown hook kind '${kind ?? ""}'`);
    return 1;
  }
  const baseUrl = resolveBaseUrl(argValue(argv.slice(1), "--base-url"));
  const token = readTokenForHook(argValue(argv.slice(1), "--token"));
  const opts: ClientOptions = { baseUrl, token: token ?? "" };

  try {
    const raw = await readStdin();
    let input: Record<string, unknown> = {};
    if (raw.trim()) {
      input = JSON.parse(raw) as Record<string, unknown>;
    }
    const event = mapHookToEvent(kind, input);
    const { id: _id, ...body } = event;
    // Keep hooks silent on success (the server already logs the event).
    await postEvent(opts, body);
  } catch (err) {
    // Hook failures must NEVER break the agent (document §26).
    error("hook forwarding failed (ignored)", err);
  }
  return 0;
}

function argValue(argv: string[], name: string): string | undefined {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
}
