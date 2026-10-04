import type { CardActionEvent } from "@larksuiteoapi/node-sdk";
import { basename } from "node:path";
import { error, log } from "../core/logger.js";
import {
  PermissionStore,
  type PermissionApi,
  type PermissionCreateInput,
  type PermissionDecision,
  type PermissionRequest,
  type PermissionWaitResult,
} from "../domain/permission.js";
import { getSendChannel } from "./feishu/channel.js";
import { buildPermissionCard } from "./feishu/card.js";
import { loadAppConfig, type AppConfig } from "./config.js";

/**
 * 权限请求交互编排：创建请求 → 发飞书卡片；卡片按钮/文本决定 → 解析 → 唤醒等待的 Hook。
 * 不依赖长连接生命周期（发卡片用独立发送 Channel；收卡片事件由 connection 回调进来）。
 */
export class PermissionService implements PermissionApi {
  readonly store: PermissionStore;

  constructor(getConfig: () => AppConfig = loadAppConfig, store?: PermissionStore) {
    this.getConfig = getConfig;
    this.store = store ?? new PermissionStore({ onTimeout: (req) => this.handleTimeout(req) });
  }

  private readonly getConfig: () => AppConfig;

  async create(input: PermissionCreateInput): Promise<{ requestId: string }> {
    const project = input.cwd ? basename(input.cwd) : undefined;
    const req = this.store.create({ ...input, project });
    // 发卡片失败不影响请求本身（Hook 会等到超时后回退终端审批流）。
    this.notifyCard(req).catch((err) => error("permission card send failed (ignored)", err));
    return { requestId: req.id };
  }

  wait(id: string, timeoutMs: number): Promise<PermissionWaitResult> {
    return this.store.waitForDecision(id, timeoutMs);
  }

  /** 处理飞书卡片按钮点击（长连接 card.action.trigger）。 */
  async handleCardAction(evt: CardActionEvent): Promise<void> {
    const value = (evt.action?.value ?? {}) as { rid?: string; act?: string };
    if (!value.rid || (value.act !== "allow" && value.act !== "deny")) return;
    const cfg = this.getConfig();
    if (!cfg.feishuOpenId || evt.operator?.openId !== cfg.feishuOpenId) {
      log({ msg: "permission card action ignored (not the bound user)" });
      return;
    }
    const req = this.store.get(value.rid);
    if (!req || req.status !== "pending") return;
    this.store.resolve(req.id, value.act);
    // 卡片更新失败不影响决定（决定已生效）。
    this.updateCard(evt.messageId, req, value.act).catch((err) => error("permission card update failed (ignored)", err));
  }

  /** 文本兜底：用户回复「允许 <id> / 拒绝 <id>」（走消息通道）。 */
  async resolveFromText(rid: string, act: PermissionDecision, senderOpenId: string): Promise<string> {
    const cfg = this.getConfig();
    if (!cfg.feishuOpenId || senderOpenId !== cfg.feishuOpenId) {
      return "⚠️ 只有绑定用户才能操作。";
    }
    const req = this.store.get(rid);
    if (!req) return `❌ 找不到请求 ${rid}。`;
    if (req.status !== "pending") return `ℹ️ 请求 ${rid} 已处理（${req.status}）。`;
    this.store.resolve(req.id, act);
    if (req.cardMessageId) {
      this.updateCard(req.cardMessageId, req, act).catch((err) => error("update card failed", err));
    }
    return act === "allow" ? `✅ 已允许 ${rid}` : `⛔ 已拒绝 ${rid}`;
  }

  private async notifyCard(req: PermissionRequest): Promise<void> {
    const cfg = this.getConfig();
    if (!cfg.feishuAppId || !cfg.feishuAppSecret) {
      log({ msg: "permission card skipped (feishu not configured)" });
      return;
    }
    const receiveId = cfg.feishuOpenId ?? cfg.feishuChatId;
    if (!receiveId) {
      log({ msg: "permission card skipped (no bound user/chat)" });
      return;
    }
    const channel = getSendChannel(cfg.feishuAppId, cfg.feishuAppSecret);
    const res = await channel.send(receiveId, { card: buildPermissionCard(req, "pending") });
    req.cardMessageId = res.messageId;
    log({ msg: "permission card sent", requestId: req.id, agent: req.agent, tool: req.toolName });
  }

  /** 请求超时（TTL 到）时把卡片更新为超时状态。 */
  private handleTimeout(req: PermissionRequest): void {
    if (!req.cardMessageId) return;
    this.updateCard(req.cardMessageId, req, undefined, "timeout").catch((err) =>
      error("permission timeout card update failed (ignored)", err)
    );
  }

  private async updateCard(
    messageId: string,
    req: PermissionRequest,
    act?: PermissionDecision,
    state: "decided" | "timeout" = "decided"
  ): Promise<void> {
    const cfg = this.getConfig();
    if (!cfg.feishuAppId || !cfg.feishuAppSecret) return;
    const channel = getSendChannel(cfg.feishuAppId, cfg.feishuAppSecret);
    await channel.updateCard(messageId, buildPermissionCard(req, state, act));
  }

  stop(): void {
    this.store.stop();
  }
}
