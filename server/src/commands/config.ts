import { loadAppConfig, saveAppConfig, type AppConfig } from "../features/config.js";
import { argValue, mask } from "./util.js";

export async function cmdConfig(argv: string[]): Promise<number> {
  const webhook = argValue(argv, "--feishu-webhook");
  const secret = argValue(argv, "--feishu-secret");
  const appId = argValue(argv, "--feishu-app-id");
  const appSecret = argValue(argv, "--feishu-app-secret");
  const chatId = argValue(argv, "--feishu-chat-id");
  if (argv.includes("--show")) {
    const cfg = loadAppConfig();
    console.log(`feishuMode=${cfg.feishuAppId ? "自建应用机器人" : cfg.feishuWebhook ? "Webhook 机器人" : "未配置"}`);
    console.log(`feishuAppId=${cfg.feishuAppId ?? "(未设置)"}`);
    console.log(`feishuAppSecret=${mask(cfg.feishuAppSecret)}`);
    console.log(`feishuChatId=${cfg.feishuChatId ?? "(未设置)"}`);
    console.log(`feishuOpenId=${cfg.feishuOpenId ?? "(未设置)"}`);
    console.log(`feishuMobile=${cfg.feishuMobile ?? "(未设置)"}`);
    console.log(`feishuWebhook=${cfg.feishuWebhook ?? "(未设置)"}`);
    console.log(`feishuSecret=${mask(cfg.feishuSecret)}`);
    return 0;
  }
  if (argv.includes("--clear-feishu")) {
    const cfg = loadAppConfig();
    const keys: (keyof AppConfig)[] = ["feishuWebhook", "feishuSecret", "feishuAppId", "feishuAppSecret", "feishuChatId", "feishuOpenId", "feishuMobile"];
    for (const k of keys) delete cfg[k];
    saveAppConfig(cfg);
    console.log("feishu config cleared");
    return 0;
  }
  if (appId || appSecret || chatId) {
    const cfg = loadAppConfig();
    if (appId) cfg.feishuAppId = appId;
    if (appSecret) cfg.feishuAppSecret = appSecret;
    if (chatId) cfg.feishuChatId = chatId;
    saveAppConfig(cfg);
    console.log("feishu app bot config saved");
    return 0;
  }
  const openId = argValue(argv, "--feishu-open-id");
  const mobile = argValue(argv, "--feishu-mobile");
  if (openId || mobile) {
    const cfg = loadAppConfig();
    if (openId) cfg.feishuOpenId = openId;
    if (mobile) cfg.feishuMobile = mobile;
    saveAppConfig(cfg);
    console.log("feishu open_id saved");
    return 0;
  }
  if (webhook) {
    const cfg = loadAppConfig();
    cfg.feishuWebhook = webhook;
    if (secret) cfg.feishuSecret = secret;
    else if (argv.includes("--feishu-secret")) cfg.feishuSecret = "";
    saveAppConfig(cfg);
    console.log("feishu webhook saved");
    return 0;
  }
  console.error("usage: ai-task-notify config --feishu-app-id <id> --feishu-app-secret <secret> --feishu-chat-id <chatId> | --feishu-webhook <url> [--feishu-secret <secret>] | --show | --clear-feishu");
  return 1;
}
