# 架构

## 总体链路

```
Codex CLI ──Hook──┐
                 ├─▶ bin/ai-task-notify hook <kind>
Claude Code ─Hook─┘        │ HTTP POST /api/events（Bearer Token）
                           ▼
              ┌─────────────────────────────┐
              │  transport/server           │  链路层：只做鉴权 + 路由
              │   → domain/normalize        │  业务层：校验/标准化为 AgentEvent
              │   → domain/pipeline         │  业务层：事件编排（扇出）
              │       ├─ log handler        │
              │       ├─ history handler    │  功能层：落盘 events.jsonl
              │       └─ notifier handler   │  功能层：读配置→选渠道→发送
              └────────────┬────────────────┘
                           ▼
              ┌────────────┴────────────────────────────┐
              │ features/feishu（官方 Channel SDK）        │
              │  ├─ 发送：channel.send（im/v1/messages）    │
              │  └─ 接收：connection（WebSocket 长连接）     │
              │       └─ bot（命令 / 自动绑定单聊）          │
              └────────────┬────────────────────────────┘
                           ▼
                     飞书单聊（你的手机/手表）
```


## 分层职责

| 层 | 目录 | 职责 | 禁止 |
|---|---|---|---|
| 入口 | `index.ts` + `commands/` | CLI 命令分发、参数解析 | 不写业务逻辑 |
| 链路 | `transport/` | HTTP/WS 服务、Hook 解析、HTTP 客户端、mDNS | 不碰业务、不碰飞书 |
| 业务 | `domain/` | 统一事件模型、校验、Pipeline 编排 | 不认识 HTTP、不认识飞书 |
| 功能 | `features/` | 通知（飞书渠道/渲染/查询）、历史落盘、配置 | 相互不依赖，只通过 Pipeline 消费事件 |
| 基础 | `core/` | 鉴权、日志、token/状态目录、版本 | 谁都能用，不依赖上层 |

## 核心机制：权限请求交互（飞书卡片远程控制）

```
Agent Hook --wait ──POST──▶ /api/permission-requests ──▶ PermissionService
                                                          │ 发交互卡片
                                                          ▼
                                                     飞书卡片【允许/拒绝】
                                                          │ card.action.trigger（长连接）
                                                          ▼
                                                   PermissionService 校验本人 → resolve
                                                          │ 唤醒长轮询
                                                          ▼
                                               Hook 拿到决定 → stdout JSON → Agent 继续/拦截
```

- 请求存内存（TTL 10min），Hook 超时/失败一律静默 exit 0 → Agent 回退终端审批流。
- 只有绑定的 open_id 能决定；文本兜底「允许/拒绝 <id>」走消息通道。

## 核心机制：Pipeline

```
一条 AgentEvent 进来
    ▼
Pipeline.handle(event)
    ├─→ log handler      只打日志
    ├─→ history handler  只写 events.jsonl
    └─→ notifier handler 只决定"要不要发、发给谁"（飞书渠道在 features/feishu）
```

- 每个 handler 独立，失败各自兜底（记录日志），**互不影响、绝不阻断 Agent**。
- 以后加新通知渠道（钉钉/邮件/短信），只需新增一个 handler 注册进 Pipeline，其它代码零改动。

## 模块文件地图

```
server/src/
├── index.ts                命令分发（≈90 行）
├── commands/               server / config / feishu / hook / status / test
├── domain/                 types（事件模型）、normalize（校验）、pipeline（编排）、permission（权限请求模型+Store）、log-handler
├── transport/              server（HTTP+WS）、ws（Hub）、hooks（Hook 解析）、client、mdns
├── features/               notifier（通知编排）、history（落盘）、config（配置读写）
│   ├── permission.ts       权限请求交互编排（发卡片 / 决定 / 文本兜底）
│   └── feishu/             channel（官方 Channel SDK 发送）、connection（长连接接收+卡片回调）、bot（命令/自动绑定/文本兜底）、card（交互卡片）、channels（发送编排）、render（文本）、queries（open_id/群列表）、api（查询用 token）
└── core/                   auth、logger、state（token/状态目录）、version

macos/
├── ConfigStore.swift       数据层：config.json / token / events.jsonl
├── ServerController.swift  进程层：启动/停止 Server、运行内置 CLI
├── AppMenuView.swift       UI 层：菜单栏弹窗（只渲染 + 收集配置）
└── AITaskNotifyApp.swift   @main 入口（≈40 行）
```

## 可靠性

- Hook 命令永远 `exit 0`，失败只记录日志，绝不阻断 Codex / Claude。
- Server 挂了只影响通知，不影响 Agent 任务。
- 通知渠道失败（飞书 API 报错）只记录日志，不阻塞事件管线。
- mDNS（Bonjour）注册失败不影响 Server 本身。

## 安全

- 默认监听 `0.0.0.0:3210`，但 HTTP 与 WebSocket 都必须带 Token。
- Token 来源优先级：`--token` > `AI_TASK_NOTIFY_TOKEN` > `~/.ai-task-notify/token`（首次自动生成，0600 权限）。
- 日志不记录 Token、密钥、完整 Hook payload。
- 飞书配置存于 `~/.ai-task-notify/config.json`（0600），不进 Git。
