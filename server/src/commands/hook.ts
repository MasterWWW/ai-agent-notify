import { error } from "../core/logger.js";
import { readTokenForHook } from "../core/state.js";
import { mapHookToEvent, type HookKind } from "../transport/hooks.js";
import { postEvent, postPermissionRequest, resolveBaseUrl, waitPermissionDecision, type ClientOptions, type PermissionCreateInput } from "../transport/client.js";

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

/** 从 Hook 事件 JSON 提取权限请求（Codex/Claude PermissionRequest 事件）。 */
function extractPermissionInput(agent: "codex" | "claude", input: Record<string, unknown>): PermissionCreateInput {
  const toolName = String(input.tool_name ?? input.toolName ?? "unknown").slice(0, 100);
  const toolInput = (input.tool_input ?? input.toolInput ?? {}) as Record<string, unknown>;
  const command = typeof toolInput.command === "string" ? toolInput.command : undefined;
  const description = typeof toolInput.description === "string" ? toolInput.description : undefined;
  const cwd = typeof input.cwd === "string" ? input.cwd : process.cwd();
  return { agent, toolName, command: command ?? description, cwd };
}

/** 输出决定 JSON 到 stdout（Codex 与 Claude 格式一致）。 */
function printDecision(behavior: "allow" | "deny"): void {
  const decision: Record<string, unknown> = { behavior };
  if (behavior === "deny") decision.message = "你在飞书拒绝了该操作";
  console.log(JSON.stringify({ hookSpecificOutput: { hookEventName: "PermissionRequest", decision } }));
}

/**
 * 阻断式权限交互：POST 请求到本地 Server（发飞书卡片）→ 长轮询等待决定
 * → stdout 输出 allow/deny JSON。超时/失败一律 exit 0 且不输出决定，
 * Agent 会回退到终端审批流，绝不被我们阻断。
 */
async function cmdHookPermissionWait(kind: "codex-permission" | "claude-permission", opts: ClientOptions): Promise<number> {
  try {
    const raw = await readStdin();
    const input = raw.trim() ? (JSON.parse(raw) as Record<string, unknown>) : {};
    const agent = kind === "codex-permission" ? "codex" : "claude";
    const body = extractPermissionInput(agent, input);
    const { requestId } = await postPermissionRequest(opts, body);
    // 留出余量：Hook 自身 timeout 配置建议 570s，这里最多等 540s。
    const result = await waitPermissionDecision(opts, requestId, 540_000);
    if (result.status === "allowed") printDecision("allow");
    else if (result.status === "denied") printDecision("deny");
    // timeout / 其他状态：不输出决定 → 终端审批流兜底
  } catch {
    // 完全静默：失败=无决定输出=Agent 回退终端审批流（绝不阻断/绝不报错）。
  }
  return 0;
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
  const rest = argv.slice(1);
  const baseUrl = resolveBaseUrl(argValue(rest, "--base-url"));
  const token = readTokenForHook(argValue(rest, "--token"));
  const opts: ClientOptions = { baseUrl, token: token ?? "" };

  // 阻断式权限交互模式：`hook codex-permission --wait` / `hook claude-permission --wait`
  if ((kind === "codex-permission" || kind === "claude-permission") && rest.includes("--wait")) {
    return cmdHookPermissionWait(kind, opts);
  }

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
