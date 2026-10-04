import type { PermissionRequest, PermissionDecision } from "../../domain/permission.js";

const AGENT_LABEL: Record<string, string> = { codex: "Codex", claude: "Claude" };

const CMD_LIMIT = 2000;

function shortCommand(command: string): string {
  const c = command.length > CMD_LIMIT ? `${command.slice(0, CMD_LIMIT)}…` : command;
  return c;
}

/**
 * 构建飞书交互卡片：
 * - pending：橙色标题 + 请求详情 + 【允许】【拒绝】按钮（value 带 rid + act）
 * - decided：绿色/红色标题 + 结果，无按钮
 */
export function buildPermissionCard(
  request: PermissionRequest,
  state: "pending" | "decided",
  decision?: PermissionDecision
): object {
  const agent = AGENT_LABEL[request.agent] ?? request.agent;

  const header =
    state === "pending"
      ? { title: { tag: "plain_text", content: `⚠️ ${agent} 请求执行操作` }, template: "orange" }
      : decision === "allow"
        ? { title: { tag: "plain_text", content: `✅ ${agent} 操作已允许` }, template: "green" }
        : { title: { tag: "plain_text", content: `⛔ ${agent} 操作已拒绝` }, template: "red" };

  const elements: object[] = [];
  const meta: string[] = [];
  if (request.project) meta.push(`**项目**：${request.project}`);
  if (request.cwd) meta.push(`**目录**：${request.cwd}`);
  meta.push(`**工具**：${request.toolName || "unknown"}`);
  elements.push({ tag: "div", text: { tag: "lark_md", content: meta.join("\n") } });

  if (request.command) {
    elements.push({
      tag: "div",
      text: { tag: "lark_md", content: `\`\`\`\n${shortCommand(request.command)}\n\`\`\`` },
    });
  }

  if (state === "pending") {
    elements.push({
      tag: "action",
      actions: [
        {
          tag: "button",
          text: { tag: "plain_text", content: "允许" },
          type: "primary",
          value: { rid: request.id, act: "allow" },
        },
        {
          tag: "button",
          text: { tag: "plain_text", content: "拒绝" },
          type: "danger",
          value: { rid: request.id, act: "deny" },
        },
      ],
    });
  } else {
    elements.push({
      tag: "div",
      text: {
        tag: "lark_md",
        content: decision === "allow" ? "✅ 你已在飞书允许该操作。" : "⛔ 你已在飞书拒绝该操作。",
      },
    });
  }

  return { config: { wide_screen_mode: true }, header, elements };
}
