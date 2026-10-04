import { error } from "../core/logger.js";
import type { AgentEvent } from "../domain/types.js";
import type { EventHandler } from "../domain/pipeline.js";
import { sendFeishu } from "./feishu/channels.js";
import type { AppConfig } from "./config.js";

/**
 * Notification orchestration: decide whether to notify, pick a channel, send.
 * Owns no transport and no Feishu API details — those live in features/feishu.
 */
export class NotifierHandler implements EventHandler {
  readonly name = "notifier";

  constructor(private readonly getConfig: () => AppConfig) {}

  handle(event: AgentEvent): void {
    const cfg = this.getConfig();
    if (!cfg.feishuAppId && !cfg.feishuWebhook) return;
    sendFeishu(
      {
        webhook: cfg.feishuWebhook,
        webhookSecret: cfg.feishuSecret,
        appId: cfg.feishuAppId,
        appSecret: cfg.feishuAppSecret,
        chatId: cfg.feishuChatId,
        openId: cfg.feishuOpenId,
      },
      event
    ).catch((err) => error("feishu send failed (ignored)", err));
  }
}
