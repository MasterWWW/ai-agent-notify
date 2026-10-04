# AI Task Notify

## 1. 项目目标

开发一个运行在 macOS 上的轻量级 AI Agent 任务通知服务，用于接收 Codex CLI、Claude Code 等 AI Coding Agent 的任务事件，并通过局域网 WebSocket 将事件推送到 vivo 手机。

vivo 手机收到事件后，通过 Android 系统 Notification 产生通知，由 vivo WATCH 5 同步显示。

### 第一阶段最终链路

```text
Codex CLI
    │
    │ Hook
    ▼
┌─────────────────────┐
│   AI Task Notify    │
│                     │
│  HTTP API / Event   │
│       Hub           │
└─────────┬───────────┘
          │
          │ WebSocket
          │ LAN
          ▼
┌─────────────────────┐
│    vivo Android     │
│        App          │
│                     │
│  WebSocket Client   │
│        ↓            │
│ Android Notification│
└─────────┬───────────┘
          │
          │ 手机系统通知
          ▼
┌─────────────────────┐
│    vivo WATCH 5     │
└─────────────────────┘
```

---

# 2. 第一阶段范围

第一阶段只实现以下能力：

### Mac Server

- HTTP API
- WebSocket Server
- Agent Event 接收
- Event 标准化
- WebSocket 广播
- 客户端连接管理
- 基础鉴权
- 日志
- 健康检查

### Codex

通过 Codex Hook 接入：

- `Stop`
- `PermissionRequest`
- `SubagentStop`（如果当前 Codex 版本支持）

### Claude Code

通过 Claude Code Hook 接入：

- `Stop`
- `Notification`
- `PermissionRequest`

### Android

开发一个极简 vivo 手机 App：

- WebSocket 连接 Mac
- 自动重连
- Android Notification
- 显示 Agent / 项目 / 状态 / 消息
- 显示连接状态
- App 前台可以查看最近事件

### vivo WATCH 5

第一阶段不开发 Watch App。

只利用 vivo 手机系统通知同步能力。

---

# 3. 明确不做的内容

第一阶段禁止过度设计。

暂时不做：

- 云服务器
- 公网推送
- 钉钉
- 微信
- 企业内部应用
- 第三方 Push 服务
- 用户账号系统
- 数据库
- 多用户
- Web 管理后台
- Watch 独立 App
- Watch 端交互
- Agent 任务控制
- 远程执行 Codex
- 远程执行 Claude
- 权限批准/拒绝的 Watch 操作

第一阶段的目标只有：

> **可靠地把 AI Agent 的状态通知送到 vivo WATCH 5。**

---

# 4. 技术架构

## 4.1 总体架构

```text
                         Mac
┌───────────────────────────────────────────┐
│                                           │
│  ┌─────────────┐     ┌────────────────┐  │
│  │ Codex CLI   │────▶│                │  │
│  └─────────────┘     │                │  │
│                      │ AI Task Notify │  │
│  ┌─────────────┐     │                │  │
│  │ Claude Code │────▶│ HTTP API       │  │
│  └─────────────┘     │ Event Hub      │  │
│                      │ WebSocket      │  │
│                      │                │  │
│                      └───────┬────────┘  │
│                              │           │
└──────────────────────────────┼───────────┘
                               │
                         LAN WebSocket
                               │
                               ▼
                    ┌─────────────────────┐
                    │     vivo Phone      │
                    │                     │
                    │ WebSocket Client    │
                    │         │           │
                    │         ▼           │
                    │ Android Notification│
                    └─────────┬───────────┘
                              │
                              │
                              ▼
                       vivo WATCH 5
```

---

# 5. 项目结构

建议使用 Monorepo。

```text
ai-task-notify/
├── apps/
│   ├── server/
│   │   ├── src/
│   │   │   ├── server/
│   │   │   ├── websocket/
│   │   │   ├── events/
│   │   │   ├── auth/
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   └── android/
│       ├── app/
│       └── build.gradle.kts
│
├── packages/
│   └── protocol/
│       ├── src/
│       │   ├── event.ts
│       │   └── message.ts
│       └── package.json
│
├── integrations/
│   ├── codex/
│   │   ├── hooks/
│   │   └── README.md
│   │
│   └── claude/
│       ├── hooks/
│       └── README.md
│
├── docs/
│   ├── architecture.md
│   ├── protocol.md
│   ├── codex.md
│   ├── claude.md
│   └── android.md
│
├── package.json
├── pnpm-workspace.yaml
└── README.md
```

如果 Codex 判断 Monorepo 对第一阶段明显过重，可以调整成：

```text
server/
android/
integrations/
```

但必须保持 Server、协议、Agent Integration、Android 的职责分离。

---

# 6. Event 协议

所有 Agent 进入 AI Task Notify 后，必须转换成统一 Event。

## 6.1 基础事件

```ts
interface AgentEvent {
  id: string;

  agent: "codex" | "claude";

  event:
    | "task_completed"
    | "permission_required"
    | "notification"
    | "subagent_completed";

  status: "success" | "waiting" | "info";

  project?: string;

  cwd?: string;

  branch?: string;

  title: string;

  message?: string;

  timestamp: number;

  metadata?: Record<string, unknown>;
}
```

---

# 7. Codex 映射

Codex Hook 事件转换为统一 Event。

## Stop

```text
Codex Stop
    ↓
task_completed
    ↓
status = success
```

示例：

```json
{
  "agent": "codex",
  "event": "task_completed",
  "status": "success",
  "project": "ddc-fe-admin",
  "title": "Codex 任务完成",
  "message": "菜单公共组件重构完成"
}
```

## PermissionRequest

```text
Codex PermissionRequest
        ↓
permission_required
        ↓
status = waiting
```

示例：

```json
{
  "agent": "codex",
  "event": "permission_required",
  "status": "waiting",
  "project": "ddc-fe-admin",
  "title": "Codex 等待权限",
  "message": "需要执行 Bash 命令"
}
```

## SubagentStop

如果当前 Codex 版本支持：

```text
SubagentStop
    ↓
subagent_completed
```

不要因为版本不支持而影响主流程。

---

# 8. Claude Code 映射

## Stop

```text
Claude Stop
    ↓
task_completed
```

## Notification

```text
Claude Notification
    ↓
notification
```

## PermissionRequest

```text
Claude PermissionRequest
    ↓
permission_required
    ↓
status = waiting
```

Claude Code 的不同 Notification 类型需要做过滤。

避免同一事件产生重复通知。

---

# 9. HTTP API

第一阶段只需要几个 API。

## 健康检查

```http
GET /health
```

返回：

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

---

## 接收事件

```http
POST /api/events
```

请求：

```json
{
  "agent": "codex",
  "event": "task_completed",
  "status": "success",
  "project": "ddc-fe-admin",
  "title": "Codex 任务完成",
  "message": "菜单重构完成",
  "timestamp": 1791111111111
}
```

返回：

```json
{
  "success": true,
  "eventId": "evt_xxx"
}
```

---

# 10. WebSocket

WebSocket：

```text
ws://<mac-ip>:3210/ws
```

例如：

```text
ws://192.168.1.100:3210/ws
```

手机启动后主动连接。

Server：

```text
Client connected
        ↓
保存 WebSocket connection
        ↓
等待 Event
```

收到事件：

```text
HTTP POST /api/events
        ↓
Event Hub
        ↓
broadcast()
        ↓
所有 WebSocket clients
```

第一阶段虽然只有一个手机，但 Server 设计上允许多个客户端。

---

# 11. WebSocket 消息格式

统一：

```json
{
  "type": "agent_event",
  "payload": {
    "id": "evt_xxx",
    "agent": "codex",
    "event": "task_completed",
    "status": "success",
    "project": "ddc-fe-admin",
    "title": "Codex 任务完成",
    "message": "菜单重构完成",
    "timestamp": 1791111111111
  }
}
```

连接成功：

```json
{
  "type": "connected",
  "serverVersion": "0.1.0"
}
```

心跳：

```json
{
  "type": "ping"
}
```

响应：

```json
{
  "type": "pong"
}
```

---

# 12. WebSocket 重连

Android 必须实现自动重连。

建议：

```text
第一次：1 秒
第二次：2 秒
第三次：4 秒
第四次：8 秒
最大：30 秒
```

使用 exponential backoff。

连接成功后：

```text
重置 retry counter
```

不要无限快速重连。

---

# 13. Server 鉴权

虽然只在局域网使用，也不要完全裸奔。

第一阶段使用简单 Token。

启动：

```bash
ai-task-notify server --token xxxx
```

HTTP：

```http
Authorization: Bearer xxxx
```

WebSocket：

```text
ws://192.168.1.100:3210/ws?token=xxxx
```

Token 不写死在代码中。

支持环境变量：

```bash
AI_TASK_NOTIFY_TOKEN=xxxx
```

---

# 14. Android App

第一阶段使用原生 Android。

推荐：

```text
Kotlin
Jetpack Compose
OkHttp WebSocket
Android NotificationManager
```

不要引入过多第三方框架。

---

# 15. Android 首页

首页只需要：

```text
AI Task Notify

Mac
192.168.1.100:3210

● Connected

Token
••••••••••

[ Connect ]

Recent Events

✓ Codex
ddc-fe-admin
任务完成
2 minutes ago
```

状态：

```text
● Connected
○ Disconnected
◌ Connecting
```

---

# 16. Android Notification

收到：

```json
{
  "agent": "codex",
  "event": "task_completed",
  "status": "success",
  "project": "ddc-fe-admin",
  "title": "Codex 任务完成",
  "message": "菜单重构完成"
}
```

手机通知：

```text
🤖 Codex

ddc-fe-admin

✅ 任务完成
菜单重构完成
```

Permission：

```text
🤖 Codex

ddc-fe-admin

🟡 等待权限

需要执行 Bash 命令
```

Claude：

```text
🤖 Claude Code

ddc-order-biz

✅ 任务完成
```

---

# 17. Android 通知 Channel

至少创建：

```text
AI Task Notify
```

建议分两个 Channel：

```text
AI Task
AI Permission
```

其中 Permission Channel 设置为较高重要性。

这样用户可以单独控制：

```text
任务完成
权限请求
```

---

# 18. vivo 手机适配

需要验证：

1. App 是否可以正常保持 WebSocket。
2. vivo 系统是否会限制后台连接。
3. 是否需要加入后台运行/自启动白名单。
4. 是否需要关闭电池优化。
5. vivo 健康是否能同步该 App 通知。
6. WATCH 5 是否能够显示通知标题和正文。

不要假设后台 WebSocket 永远可靠。

如果 vivo 系统杀掉后台连接，需要研究：

```text
Foreground Service
```

是否是第一阶段必要方案。

优先使用最简单实现验证。

---

# 19. WATCH 5 验证

第一阶段不开发 Watch。

验证：

```text
Android Notification
        ↓
vivo 健康
        ↓
WATCH 5
```

测试三种通知：

### 成功

```text
🤖 Codex

ddc-fe-admin

✅ 任务完成
```

### 权限

```text
🤖 Claude Code

ddc-order-biz

🟡 等待权限
```

### 普通通知

```text
🤖 Codex

任务状态更新
```

确认 WATCH 5 都能正常显示。

---

# 20. Agent Hook 接入策略

不要复制 GitHub 项目的完整实现。

重点参考：

- `wmzspace/AgentNotification`
- `ch040602/codex-cli-notify`

这些项目主要用于验证 Hook 事件和事件 Payload 的处理方式。`AgentNotification` 已经实现 Codex、Claude Code 多 Agent Hook 统一通知；`codex-cli-notify` 则专注于 Codex Hook 的本地通知。

本项目只复用设计思路，不直接复制其通知后端。

---

# 21. Codex Integration

Codex Hook 应该调用一个本地 CLI：

```bash
ai-task-notify hook codex-stop
```

而不是让 Codex Hook 自己实现 HTTP/WebSocket。

结构：

```text
Codex
 ↓
Hook
 ↓
ai-task-notify hook codex-stop
 ↓
POST http://127.0.0.1:3210/api/events
```

这样：

- Codex Hook 只负责事件采集。
- CLI 负责解析。
- Server 负责广播。
- Android 负责通知。

职责清晰。

---

# 22. Claude Integration

同样：

```text
Claude Code
 ↓
Hook
 ↓
ai-task-notify hook claude-stop
 ↓
POST /api/events
```

不要让 Claude Hook 直接连接 Android。

---

# 23. CLI

第一阶段提供：

```bash
ai-task-notify server
```

启动 Server。

```bash
ai-task-notify status
```

查看 Server 状态。

```bash
ai-task-notify test
```

发送测试通知。

```bash
ai-task-notify hook codex-stop
```

处理 Codex Stop Hook。

```bash
ai-task-notify hook codex-permission
```

处理 Codex Permission Hook。

```bash
ai-task-notify hook claude-stop
```

处理 Claude Stop Hook。

```bash
ai-task-notify hook claude-permission
```

处理 Claude Permission Hook。

---

# 24. 第一阶段 MVP 开发顺序

必须按照下面顺序执行。

## Phase 1：Server

先完成：

```text
HTTP Server
WebSocket Server
Event Protocol
Token Auth
Health Check
```

测试：

```bash
curl http://127.0.0.1:3210/health
```

然后：

```bash
curl -X POST http://127.0.0.1:3210/api/events
```

确认 Event Hub 工作。

---

## Phase 2：WebSocket Client

开发 Android App。

首先不接 Codex。

Mac：

```text
POST /api/events
```

手机：

```text
WebSocket
```

收到：

```text
Notification
```

先完成：

```text
Mac → Android → Notification
```

---

## Phase 3：WATCH 5

确认：

```text
Android Notification
        ↓
WATCH 5
```

这是第一阶段最重要的验收节点。

---

## Phase 4：Codex

接入：

```text
Stop
PermissionRequest
```

完成：

```text
Codex
 ↓
Hook
 ↓
AI Task Notify
 ↓
WebSocket
 ↓
vivo
 ↓
WATCH 5
```

---

## Phase 5：Claude Code

接入：

```text
Stop
Notification
PermissionRequest
```

完成：

```text
Claude
 ↓
Hook
 ↓
AI Task Notify
 ↓
WebSocket
 ↓
vivo
 ↓
WATCH 5
```

---

# 25. MVP 验收标准

必须全部满足：

### Server

- [ ] Server 可以启动。
- [ ] `/health` 正常。
- [ ] `/api/events` 可以接收 Event。
- [ ] WebSocket 可以连接。
- [ ] Token 鉴权生效。
- [ ] Event 可以广播。

### Android

- [ ] 可以配置 Mac IP。
- [ ] 可以配置 Token。
- [ ] 可以连接 WebSocket。
- [ ] 断开后自动重连。
- [ ] 收到 Event 后产生系统通知。
- [ ] 可以显示最近事件。
- [ ] 可以查看连接状态。

### Codex

- [ ] Stop 可以产生通知。
- [ ] PermissionRequest 可以产生通知。
- [ ] Hook 异常不会影响 Codex 正常工作。

### Claude

- [ ] Stop 可以产生通知。
- [ ] PermissionRequest 可以产生通知。
- [ ] Notification 可以产生通知。
- [ ] Hook 异常不会影响 Claude Code 正常工作。

### vivo WATCH 5

- [ ] Codex 完成通知可以显示。
- [ ] Codex 权限通知可以显示。
- [ ] Claude 完成通知可以显示。
- [ ] Claude 权限通知可以显示。

---

# 26. 非功能要求

## 可靠性

通知失败不能阻断 Agent。

例如：

```text
AI Task Notify Server 挂了
```

不能导致：

```text
Codex 任务失败
```

Hook 必须：

```text
try
  send notification
catch
  log error
  exit 0
```

---

## 性能

通知目标：

```text
Agent Event
 ↓
Server
 ↓
WebSocket
 ↓
Android Notification
```

局域网情况下尽量控制在：

```text
< 1 秒
```

---

## 安全

只允许局域网访问。

Server 默认监听：

```text
0.0.0.0:3210
```

但必须使用 Token。

不要：

- 把 Token 写进 Git。
- 把 Token 写进 Android APK。
- 将服务暴露到公网。
- 默认开放无需认证的 WebSocket。

---

# 27. 日志

Server 日志至少记录：

```text
timestamp
eventId
agent
event
project
client count
error
```

例如：

```text
2026-10-04 21:30:12
event=task_completed
agent=codex
project=ddc-fe-admin
clients=1
```

不要记录：

- API Key
- Token
- Secret
- 完整敏感代码
- 完整 Hook Payload

---

# 28. 后续扩展

第一阶段完成以后，再考虑：

## Jenkins

```text
Jenkins
 ↓
AI Task Notify
 ↓
WATCH 5
```

例如：

```text
🚀 Jenkins

ddc-fe-admin

✅ Deploy success
```

## Git

```text
git push
 ↓
AI Task Notify
```

## 其他 Agent

未来可以支持：

```text
Gemini CLI
Kimi CLI
Aider
OpenCode
自定义 CLI
```

通过统一 Adapter：

```text
Agent
 ↓
Adapter
 ↓
AgentEvent
```

---

# 29. 后续 Watch 交互

不要放在第一阶段。

未来可以考虑：

```text
WATCH 5

Claude
🟡 Waiting

[查看]
```

甚至：

```text
[Approve]
[Reject]
```

但这需要重新设计：

```text
WATCH
 ↓
Phone
 ↓
WebSocket
 ↓
Mac
 ↓
Agent
```

这是第二阶段功能。

第一阶段只做：

```text
Mac → Phone → Watch
```

不要反向控制。

---

# 30. 开发原则

Codex 执行本项目时必须遵守：

1. **先检查当前环境和项目实际情况，再修改。**
2. 不要假设 Codex / Claude 当前版本和网上示例完全一致。
3. Hook 配置必须以当前本机实际版本为准。
4. 优先使用官方文档和当前 CLI 的实际配置格式。
5. 不直接复制第三方项目代码。
6. 第三方 GitHub 项目只作为架构和兼容性参考。
7. 保持实现简单。
8. 不提前实现数据库。
9. 不提前实现账号体系。
10. 不提前实现云端 Push。
11. 不提前实现 Watch App。
12. 不为了“未来扩展”引入大型框架。
13. 所有 Hook 失败不得阻断 Codex / Claude。
14. 所有配置变更必须可回滚。
15. 修改 Codex / Claude 配置之前必须备份原配置。
16. 优先提供 dry-run / test 能力。
17. 优先完成端到端 MVP，再进行代码重构。
18. 不要一次性实现 Phase 2 功能。

---

# 31. 推荐参考项目

### AgentNotification

https://github.com/wmzspace/AgentNotification

重点参考：

- Codex Hook
- Claude Code Hook
- Agent Event 统一
- Hook 自动注入
- Hook 失败不阻断 Agent

该项目已经验证 Codex CLI、Claude Code → 手机通知的统一方案。

### Codex CLI Notify

https://github.com/ch040602/codex-cli-notify

重点参考：

- Codex `Stop`
- Codex `PermissionRequest`
- Codex `SubagentStop`
- Hook 配置
- macOS 本地通知

它本身是本地通知工具，不负责手机 Push，因此不要直接照搬其通知后端。

---

# 32. 第一阶段最终结果

完成以后，用户在 Mac 上运行：

```bash
codex
```

或者：

```bash
claude
```

执行长任务。

用户可以离开 Mac。

任务完成时：

```text
Codex
 ↓
Hook
 ↓
AI Task Notify
 ↓
LAN WebSocket
 ↓
vivo 手机
 ↓
Android Notification
 ↓
vivo WATCH 5
```

最终用户在 WATCH 5 上看到：

```text
🤖 Codex

ddc-fe-admin

✅ 任务完成
```

或者：

```text
🤖 Claude Code

ddc-order-biz

🟡 等待权限
```

这就是第一阶段的完整闭环。

---

# 33. Codex 执行要求

开始编码之前：

1. 检查当前仓库。
2. 检查 Node / pnpm / Java / Android SDK 等环境。
3. 检查当前 Codex CLI 版本。
4. 检查当前 Claude Code 版本。
5. 检查当前 Codex Hook 配置格式。
6. 检查当前 Claude Code Hook 配置格式。
7. 输出实际检测结果。
8. 根据检测结果调整实现方案。
9. 先完成 Server + WebSocket + Android 最小链路。
10. 验证 Mac → vivo 手机 → WATCH 5。
11. 再接入 Codex。
12. 最后接入 Claude Code。

**不要在没有完成前一个阶段验证之前进入下一阶段。**
