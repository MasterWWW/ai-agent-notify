import { loadAppConfig, saveAppConfig } from "../features/config.js";
import { testFeishu } from "../features/feishu/channels.js";
import { listChats, resolveOpenIdByMobile } from "../features/feishu/queries.js";
import { argValue } from "./util.js";
import { startFeishuBot } from "../features/feishu/connection.js";
import { handleBotMessage } from "../features/feishu/bot.js";
import { log } from "../core/logger.js";

export async function cmdFeishu(argv: string[]): Promise<number> {
  const sub = argv[0];
  const cfg = loadAppConfig();
  if (sub === "me") {
    const mobile = argValue(argv, "--mobile");
    if (!mobile) {
      console.error("usage: ai-task-notify feishu me --mobile <手机号>");
      return 1;
    }
    if (!cfg.feishuAppId || !cfg.feishuAppSecret) {
      console.error("需要先配置 --feishu-app-id 和 --feishu-app-secret");
      return 1;
    }
    try {
      const openId = await resolveOpenIdByMobile(cfg.feishuAppId, cfg.feishuAppSecret, mobile);
      cfg.feishuOpenId = openId;
      cfg.feishuMobile = mobile;
      saveAppConfig(cfg);
      console.log(`✅ 已找到你的 open_id=${openId}`);
      console.log("已保存：之后消息会直接发到你和机器人的单聊窗口。");
      return 0;
    } catch (err) {
      console.error(`查询失败：${err instanceof Error ? err.message : String(err)}`);
      return 1;
    }
  }
  if (sub === "test") {
    console.log(
      await testFeishu({
        webhook: cfg.feishuWebhook,
        webhookSecret: cfg.feishuSecret,
        appId: cfg.feishuAppId,
        appSecret: cfg.feishuAppSecret,
        chatId: cfg.feishuChatId,
        openId: cfg.feishuOpenId,
      })
    );
    return 0;
  }
  if (sub === "chats") {
    if (!cfg.feishuAppId || !cfg.feishuAppSecret) {
      console.error("需要先配置 --feishu-app-id 和 --feishu-app-secret");
      return 1;
    }
    try {
      const chats = await listChats(cfg.feishuAppId, cfg.feishuAppSecret);
      if (chats.length === 0) {
        console.log("（没有找到机器人所在的群）");
        return 0;
      }
      for (const c of chats) console.log(`${c.chat_id}\t${c.name}`);
      return 0;
    } catch (err) {
      console.error(`获取群列表失败：${err instanceof Error ? err.message : String(err)}`);
      return 1;
    }
  }
  if (sub === "connect") {
    if (!cfg.feishuAppId || !cfg.feishuAppSecret) {
      console.error("需要先配置 --feishu-app-id 和 --feishu-app-secret");
      return 1;
    }
    console.log("正在连接飞书长连接（WebSocket），Ctrl+C 退出……");
    try {
      const bot = await startFeishuBot(cfg.feishuAppId, cfg.feishuAppSecret, {
        onMessage: (msg) => handleBotMessage(msg, { botInfo: () => ({ connected: true }) }),
        onState: (state) => log({ msg: "feishu bot state", state }),
      });
      log({ msg: "feishu bot ready (长连接已建立，私聊机器人即可发送命令)" });
      await new Promise<void>((resolve) => {
        const onSig = () => {
          process.off("SIGINT", onSig);
          process.off("SIGTERM", onSig);
          bot.stop().then(() => resolve());
        };
        process.on("SIGINT", onSig);
        process.on("SIGTERM", onSig);
      });
      return 0;
    } catch (err) {
      console.error(`连接失败：${err instanceof Error ? err.message : String(err)}`);
      console.error("请确认开放平台已配置：事件订阅 → 添加 im.message.receive_v1 → 订阅方式选「长连接」，并已发布版本。");
      return 1;
    }
  }
  console.error("usage: ai-task-notify feishu test | feishu chats | feishu me --mobile <手机号> | feishu connect");
  return 1;
}
