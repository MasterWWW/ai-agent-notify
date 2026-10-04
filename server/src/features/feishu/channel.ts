import { createLarkChannel, LoggerLevel, type LarkChannel } from "@larksuiteoapi/node-sdk";

/**
 * 官方 Channel SDK（@larksuiteoapi/node-sdk 的 LarkChannel）统一入口。
 * 发送消息走 HTTP（im/v1/messages），接收消息走长连接（WebSocket）。
 * 这里只缓存发送用的 Channel 实例（不建立长连接，纯内存创建）。
 */
const sendChannels = new Map<string, LarkChannel>();

export function getSendChannel(appId: string, appSecret: string): LarkChannel {
  let ch = sendChannels.get(appId);
  if (!ch) {
    ch = createLarkChannel({ appId, appSecret, loggerLevel: LoggerLevel.warn });
    sendChannels.set(appId, ch);
  }
  return ch;
}

/**
 * 用自建应用机器人身份发送一条文本消息。
 * receiveId 支持 open_id(ou_)/chat_id(oc_)/union_id(on_)/user_id/email，
 * SDK 自动识别 receive_id_type；token 由 SDK 缓存并自动刷新。
 */
export async function sendAppText(
  appId: string,
  appSecret: string,
  receiveId: string,
  text: string
): Promise<void> {
  const channel = getSendChannel(appId, appSecret);
  await channel.send(receiveId, { text });
}
