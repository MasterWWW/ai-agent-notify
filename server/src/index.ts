import { VERSION } from "./core/version.js";
import { cmdServer } from "./commands/server.js";
import { cmdConfig } from "./commands/config.js";
import { cmdFeishu } from "./commands/feishu.js";
import { cmdHook, HOOK_KINDS } from "./commands/hook.js";
import { cmdStatus } from "./commands/status.js";
import { cmdTest } from "./commands/test.js";

function printUsage(): void {
  console.log(`AI Task Notify v${VERSION}

Usage:
  ai-task-notify server [--host 0.0.0.0] [--port 3210] [--token xxx]
  ai-task-notify status [--base-url http://127.0.0.1:3210]
  ai-task-notify test [--message "..."] [--project "..."] [--token xxx] [--base-url ...]
  ai-task-notify config --feishu-app-id <id> --feishu-app-secret <secret> [--feishu-chat-id <chatId>]
  ai-task-notify feishu me --mobile <手机号>   # 查你的 open_id，直接发到与机器人的单聊
  ai-task-notify config --feishu-webhook <url> [--feishu-secret <secret>]
  ai-task-notify config --show
  ai-task-notify config --clear-feishu
  ai-task-notify feishu test          # 发送测试消息，验证飞书配置
  ai-task-notify feishu chats         # 列出机器人所在的群/会话（需要 im:chat:readonly 权限）
  ai-task-notify hook <kind> [--token xxx] [--base-url ...]

Hook kinds:
  ${HOOK_KINDS.join("\n  ")}

Env:
  AI_TASK_NOTIFY_TOKEN       token (used when --token is absent)
  AI_TASK_NOTIFY_BASE_URL    server base url for hooks/test/status
`);
}

export async function main(argv: string[]): Promise<number> {
  // Tolerate a leading "--" (e.g. `pnpm start -- server`).
  if (argv[0] === "--") argv = argv.slice(1);
  const [cmd, ...rest] = argv;
  switch (cmd) {
    case "server":
      return cmdServer(rest);
    case "status":
      return cmdStatus(rest);
    case "test":
      return cmdTest(rest);
    case "config":
      return cmdConfig(rest);
    case "feishu":
      return cmdFeishu(rest);
    case "hook":
      return cmdHook(rest);
    case "version":
      console.log(VERSION);
      return 0;
    case "--help":
    case "-h":
    case undefined:
      printUsage();
      return 0;
    default:
      console.error(`ai-task-notify: unknown command '${cmd}'`);
      printUsage();
      return 1;
  }
}

// Allow direct execution: node dist/index.js server ...
function isDirectRun(): boolean {
  try {
    if (typeof require !== "undefined" && require.main === module) return true;
  } catch {
    // CJS require is unavailable under ESM; fall through.
  }
  if (!process.argv[1]) return false;
  try {
    return import.meta.url.endsWith(process.argv[1]);
  } catch {
    return false;
  }
}

if (isDirectRun()) {
  main(process.argv.slice(2)).then((code) => {
    process.exitCode = code;
  });
}
