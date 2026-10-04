# 架构

## 总体链路

```
                         Mac
┌───────────────────────────────────────────┐
│                                           │
│  Codex CLI ──Hook──▶ ┌────────────────┐   │
│                       │                │   │
│  Claude Code ──Hook──▶│ AI Task Notify │   │
│                       │ HTTP API       │   │
│                       │ WebSocket Hub  │   │
│                       │ Token Auth     │   │
│                       │ mDNS (dns-sd)  │   │
│                       └───────┬────────┘   │
└───────────────────────────────┼───────────┘
                          LAN WebSocket
                                │
                                ▼
                     vivo 手机（WebSocket Client → 系统通知）
                                │
                                ▼
                          vivo WATCH 5（系统通知同步）
```

## 职责分离

- **Agent Hook** 只负责采集事件，调用本地 CLI：`ai-task-notify hook <kind>`。
- **CLI** 负责把 Hook stdin 的原始 payload 标准化成统一 `AgentEvent` 并 POST 到本机 Server。
- **Server** 负责鉴权、广播、连接管理、日志。
- **Android** 只负责 WebSocket 连接 + 系统通知，不直接与 Agent 交互。

## 可靠性

- Hook 命令永远 `exit 0`，失败只记录日志，绝不阻断 Codex / Claude。
- Server 挂了只影响通知，不影响 Agent 任务。
- mDNS（Bonjour）注册失败不影响 Server 本身。

## 安全

- 默认监听 `0.0.0.0:3210`，但 HTTP 与 WebSocket 都必须带 Token。
- Token 来源优先级：`--token` > `AI_TASK_NOTIFY_TOKEN` > `~/.ai-task-notify/token`（首次自动生成，0600 权限）。
- 日志不记录 Token、密钥、完整 Hook payload。
