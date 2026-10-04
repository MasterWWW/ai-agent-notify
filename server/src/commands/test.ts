import { error } from "../core/logger.js";
import { readTokenForHook } from "../core/state.js";
import { postEvent, resolveBaseUrl, type ClientOptions } from "../transport/client.js";
import type { EventInput } from "../domain/types.js";
import { argValue } from "./util.js";

export async function cmdTest(argv: string[]): Promise<number> {
  const baseUrl = resolveBaseUrl(argValue(argv, "--base-url"));
  const token = readTokenForHook(argValue(argv, "--token"));
  if (!token) {
    console.error("ai-task-notify: no token. Set AI_TASK_NOTIFY_TOKEN or start the server once to generate ~/.ai-task-notify/token");
    return 1;
  }
  const opts: ClientOptions = { baseUrl, token };
  const event: EventInput = {
    agent: "codex",
    event: "task_completed",
    status: "success",
    title: "测试通知",
    message: argValue(argv, "--message") ?? "AI Task Notify 链路测试",
    project: argValue(argv, "--project") ?? "ai-task-notify",
  };
  try {
    const eventId = await postEvent(opts, event);
    console.log(`sent eventId=${eventId}`);
    return 0;
  } catch (err) {
    error("send test event failed", err);
    return 1;
  }
}
