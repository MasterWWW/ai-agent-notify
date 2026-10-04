import {
  createLarkChannel,
  LoggerLevel,
  type CardActionEvent,
  type LarkChannel,
  type NormalizedMessage,
} from "@larksuiteoapi/node-sdk";
import { error, log } from "../../core/logger.js";

export interface FeishuBotCallbacks {
  /** 收到发给机器人的消息；返回要回复的文本，返回 null 表示不回复。 */
  onMessage: (msg: NormalizedMessage) => string | null | Promise<string | null>;
  /** 收到交互卡片按钮点击（card.action.trigger，走长连接）。 */
  onCardAction?: (evt: CardActionEvent) => void | Promise<void>;
  onState?: (state: "connected" | "reconnecting") => void;
  onReady?: (channel: LarkChannel) => void;
}

export interface FeishuBot {
  channel: LarkChannel;
  stop: () => Promise<void>;
}

/**
 * 接管飞书自建应用机器人的聊天：用 App ID / App Secret 建立长连接（WebSocket），
 * 接收发给机器人的消息并回调 onMessage。不需要公网回调地址 / 域名 / HTTPS。
 * 连接失败、断线都只记录日志并自动重连，绝不导致 Server 退出。
 */
export async function startFeishuBot(
  appId: string,
  appSecret: string,
  cb: FeishuBotCallbacks
): Promise<FeishuBot> {
  const channel = createLarkChannel({
    appId,
    appSecret,
    transport: "websocket",
    loggerLevel: LoggerLevel.warn,
    handshakeTimeoutMs: 10_000,
    policy: { dmMode: "open", requireMention: false },
  });

  channel.on("message", async (msg) => {
    try {
      const reply = await cb.onMessage(msg);
      if (reply) await channel.send(msg.chatId, { text: reply });
    } catch (err) {
      error("feishu bot reply failed", err);
    }
  });
  if (cb.onCardAction) {
    channel.on("cardAction", (evt) => {
      Promise.resolve(cb.onCardAction!(evt)).catch((err) => error("feishu card action failed", err));
    });
  }
  channel.on("error", (err) => error("feishu long connection error", err));
  channel.on("reconnecting", () => {
    cb.onState?.("reconnecting");
    log({ msg: "feishu reconnecting" });
  });
  channel.on("reconnected", () => {
    cb.onState?.("connected");
    log({ msg: "feishu reconnected" });
  });

  await channel.connect();
  cb.onState?.("connected");
  cb.onReady?.(channel);
  return { channel, stop: () => channel.disconnect() };
}
