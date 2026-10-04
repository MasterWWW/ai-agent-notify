# AI Task Notify

运行在 macOS 上的轻量 AI Agent 任务通知服务：接收 Codex CLI / Claude Code 的 Hook 事件，通过**局域网 WebSocket** 推送到 vivo 手机，手机弹系统通知，再由 vivo WATCH 5 同步显示。

```
Codex CLI ──Hook──▶ AI Task Notify ──LAN WebSocket──▶ vivo 手机 ──系统通知──▶ vivo WATCH 5
Claude Code ─┘            │
                    HTTP API / WebSocket Server (0.0.0.0:3210, Token 鉴权)
```

## 本地名称访问（不用 IP）

手机端**不要填 IP**，填 Mac 的 mDNS 主机名，IP 变了也不用改手机：

```
ws://weichaoyingdeMac-mini.local:3210/ws?token=xxxx
```

- Server 启动时自动打印当前主机名和局域网 IP（仅用于首次核对）。
- Server 启动时通过 macOS 原生 `dns-sd` 注册 Bonjour 服务 `_ai-task-notify._tcp`（显示名 `AI Task Notify`），Android App 可用 NSD 直接发现 Mac，无需手填 IP/名称。
- 手机与 Mac 必须在同一局域网（同一 WiFi），且路由器需允许 mDNS（绝大多数家用路由器默认允许）。

## 快速开始

```bash
pnpm install
pnpm build

# 启动 Server（首次会自动生成 Token 并保存到 ~/.ai-task-notify/token）
bin/ai-task-notify server

# 或用环境变量指定 Token
AI_TASK_NOTIFY_TOKEN=xxxx bin/ai-task-notify server
```

启动后看到类似输出：

```
msg=server started version=0.1.0 port=3210
msg=lan hostname (use this on the phone) hostname=weichaoyingdeMac-mini.local:3210
msg=token (put this in the phone app) token=xxxxxxxx
msg=phone should connect to url=ws://weichaoyingdeMac-mini.local:3210/ws?token=xxxxxxxx
```

## CLI

```bash
ai-task-notify server            # 启动 Server（默认 0.0.0.0:3210）
ai-task-notify status            # 查看 Server 状态
ai-task-notify test              # 发送一条测试通知
ai-task-notify hook codex-stop   # Codex Stop Hook（读 stdin JSON，转发事件）
ai-task-notify hook codex-permission
ai-task-notify hook codex-subagent-stop
ai-task-notify hook claude-stop
ai-task-notify hook claude-permission
ai-task-notify hook claude-notification
```

Hook 命令保证：**任何失败都不影响 Agent 正常运行**（try/catch 后 exit 0）。

## 目录结构

```
server/           Node + TypeScript：HTTP / WebSocket / Token / mDNS / CLI
integrations/     Codex、Claude Hook 接入说明与配置
docs/             架构、协议、接入文档
android/          (Phase 2) Kotlin + Compose 手机 App
```

## 文档

- [架构](docs/architecture.md)
- [协议](docs/protocol.md)
- [Codex 接入](docs/codex.md)
- [Claude 接入](docs/claude.md)
- [Android App](docs/android.md)

## 阶段

- Phase 1 ✅ Server（HTTP + WebSocket + Token + mDNS + CLI）
- Phase 2 ⏳ Android App
- Phase 3 手机 → WATCH 5 验证
- Phase 4 Codex Hook 接入
- Phase 5 Claude Code Hook 接入
