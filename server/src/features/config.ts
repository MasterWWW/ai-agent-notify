import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { APP_CONFIG_FILE, STATE_DIR } from "../core/state.js";

/** Notification app config stored in ~/.ai-task-notify/config.json (0600). */
export interface AppConfig {
  /** 自定义机器人 Webhook（备选） */
  feishuWebhook?: string;
  feishuSecret?: string;
  /** 自建应用机器人（推荐） */
  feishuAppId?: string;
  feishuAppSecret?: string;
  feishuChatId?: string;
  /** 用户 open_id（发到与机器人的单聊） */
  feishuOpenId?: string;
  /** 手机号（仅用于查询 open_id，方便回填） */
  feishuMobile?: string;
}

export function loadAppConfig(): AppConfig {
  try {
    if (!existsSync(APP_CONFIG_FILE)) return {};
    return JSON.parse(readFileSync(APP_CONFIG_FILE, "utf8")) as AppConfig;
  } catch {
    return {};
  }
}

export function saveAppConfig(cfg: AppConfig): void {
  mkdirSync(STATE_DIR, { recursive: true });
  writeFileSync(APP_CONFIG_FILE, JSON.stringify(cfg, null, 2), { mode: 0o600 });
}
