export interface StatusInfo {
  running: boolean;
  port: number;
  hostname: string;
  token: string;
  feishu_configured: boolean;
  bot_connected: boolean;
  recent_events: string[];
}

export interface AppConfig {
  feishuWebhook?: string | null;
  feishuSecret?: string | null;
  feishuAppId?: string | null;
  feishuAppSecret?: string | null;
  feishuChatId?: string | null;
  feishuOpenId?: string | null;
  feishuMobile?: string | null;
}

export interface FeishuConfigInput {
  feishu_app_id?: string;
  feishu_app_secret?: string;
  feishu_chat_id?: string;
  feishu_webhook?: string;
  feishu_secret?: string;
}
