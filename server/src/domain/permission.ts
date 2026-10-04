import { randomBytes } from "node:crypto";

export type PermissionAgent = "codex" | "claude";
export type PermissionStatus = "pending" | "allowed" | "denied" | "timeout";
export type PermissionDecision = "allow" | "deny";

export interface PermissionCreateInput {
  agent: PermissionAgent;
  toolName: string;
  command?: string;
  cwd?: string;
  /** 项目名（由 cwd 推断，用于卡片展示）。 */
  project?: string;
}

export interface PermissionRequest extends PermissionCreateInput {
  id: string;
  status: PermissionStatus;
  createdAt: number;
  decidedAt?: number;
  decision?: PermissionDecision;
  /** 飞书卡片 message_id（点击后用于更新卡片），内部字段，不暴露给 Hook。 */
  cardMessageId?: string;
}

export interface PermissionWaitResult {
  status: PermissionStatus;
  decision?: PermissionDecision;
}

/** HTTP API 提供的最小接口（由组合根注入 transport/server）。 */
export interface PermissionApi {
  create(input: PermissionCreateInput): Promise<{ requestId: string }>;
  wait(id: string, timeoutMs: number): Promise<PermissionWaitResult>;
}

/**
 * 内存版权限请求存储：pending 请求默认 10 分钟 TTL，超时自动标记 timeout。
 * 长轮询等待通过 waiters 唤醒，不占用线程轮询。
 */
export interface PermissionStoreOptions {
  ttlMs?: number;
  sweepMs?: number;
  /** 请求被 TTL 标记为 timeout 时的回调（用于把卡片更新为超时状态）。 */
  onTimeout?: (req: PermissionRequest) => void;
}

export class PermissionStore {
  private readonly items = new Map<string, PermissionRequest>();
  private readonly waiters = new Map<string, Array<() => void>>();
  private readonly ttlMs: number;
  private readonly onTimeout?: (req: PermissionRequest) => void;
  private readonly timer?: NodeJS.Timeout;

  constructor(opts?: PermissionStoreOptions) {
    this.ttlMs = opts?.ttlMs ?? 10 * 60_000;
    this.onTimeout = opts?.onTimeout;
    const sweepMs = opts?.sweepMs ?? 60_000;
    this.timer = setInterval(() => this.sweep(), sweepMs);
    this.timer.unref?.();
  }

  create(input: PermissionCreateInput): PermissionRequest {
    const req: PermissionRequest = {
      id: `pr_${randomBytes(6).toString("hex")}`,
      ...input,
      status: "pending",
      createdAt: Date.now(),
    };
    this.items.set(req.id, req);
    return req;
  }

  get(id: string): PermissionRequest | undefined {
    return this.items.get(id);
  }

  /** 当前所有 pending 请求（用于状态展示/测试）。 */
  listPending(): PermissionRequest[] {
    return [...this.items.values()].filter((r) => r.status === "pending");
  }

  /** 记录决定并唤醒等待者；已处理/不存在的请求返回 undefined。 */
  resolve(id: string, decision: PermissionDecision): PermissionRequest | undefined {
    const req = this.items.get(id);
    if (!req || req.status !== "pending") return undefined;
    req.status = decision === "allow" ? "allowed" : "denied";
    req.decision = decision;
    req.decidedAt = Date.now();
    this.wake(id);
    return req;
  }

  /** 长轮询：请求被决定立即返回；超时返回 { status: "timeout" }。 */
  waitForDecision(id: string, timeoutMs: number): Promise<PermissionWaitResult> {
    const req = this.items.get(id);
    if (!req) return Promise.resolve({ status: "timeout" });
    if (req.status !== "pending") {
      return Promise.resolve({ status: req.status, decision: req.decision });
    }
    return new Promise<PermissionWaitResult>((resolvePromise) => {
      let settled = false;
      const timer = setTimeout(() => {
        if (settled) return;
        settled = true;
        this.removeWaiter(id, done);
        resolvePromise({ status: "timeout" });
      }, timeoutMs);
      const done = (): void => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        const r = this.items.get(id);
        resolvePromise(r ? { status: r.status, decision: r.decision } : { status: "timeout" });
      };
      const list = this.waiters.get(id) ?? [];
      list.push(done);
      this.waiters.set(id, list);
    });
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer);
  }

  private wake(id: string): void {
    const list = this.waiters.get(id) ?? [];
    this.waiters.delete(id);
    for (const done of list) done();
  }

  private removeWaiter(id: string, done: () => void): void {
    const list = this.waiters.get(id) ?? [];
    const i = list.indexOf(done);
    if (i >= 0) list.splice(i, 1);
    if (list.length === 0) this.waiters.delete(id);
  }

  private sweep(): void {
    const now = Date.now();
    for (const req of this.items.values()) {
      if (req.status === "pending" && now - req.createdAt > this.ttlMs) {
        req.status = "timeout";
        req.decidedAt = now;
        this.wake(req.id);
        this.onTimeout?.(req);
      }
    }
  }
}
