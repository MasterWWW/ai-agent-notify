# AI Task Notify

运行在 macOS 上的轻量 AI Agent 任务通知：接收 Codex CLI / Claude Code 的 Hook 事件，推送通知到**飞书群机器人**（也可通过局域网 WebSocket 推给手机 App，App 暂缓）。

```
Codex CLI ──Hook──┐
                  ├─▶ AI Task Notify（macOS 菜单栏 App / Server）
Claude Code ─Hook─┘          │
                             ├─▶ 飞书群机器人（你直接收通知）
                             └─▶ WebSocket（手机 App，暂缓）
```

## 一键启动：macOS App（推荐）

```bash
# 已安装到 /Applications/AI Task Notify.app，双击即可
open /Applications/"AI Task Notify.app"
```

- 菜单栏出现 🔔 图标，启动即自动拉起本地 Server（端口 3210）。
- 菜单里可查看：运行状态、主机名（mDNS 本地名称）、Token、最近事件。
- 一键复制连接地址 / Token；**配置飞书…** 里粘贴飞书机器人 Webhook 即可收通知。
- 数据都在 `~/.ai-task-notify/`（token、config.json、events.jsonl、server.log）。

## 本地名称访问（不用 IP）

手机端 / 客户端填 Mac 的 mDNS 主机名，IP 变了不用改：

```
ws://weichaoyingdeMac-mini.local:3210/ws?token=xxxx
```

Server 启动时自动打印主机名与局域网 IP，并用 `dns-sd` 注册 Bonjour 服务 `_ai-task-notify._tcp`（Android NSD 可自动发现，App 暂缓）。

## 飞书通知

```bash
# 配置飞书机器人 Webhook（群设置 → 群机器人 → 自定义机器人）
bin/ai-task-notify config --feishu-webhook "https://open.feishu.cn/open-apis/bot/v2/hook/xxx" [--feishu-secret "密钥"]
bin/ai-task-notify test   # 发送测试通知
```

详见 [docs/feishu.md](docs/feishu.md)。

## CLI（可选）

```bash
bin/ai-task-notify server            # 直接跑 Server（不进 App 时）
bin/ai-task-notify status
bin/ai-task-notify test
bin/ai-task-notify hook codex-stop   # Codex / Claude Hook 转发（失败永不阻断 Agent）
bin/ai-task-notify hook codex-permission
bin/ai-task-notify hook codex-subagent-stop
bin/ai-task-notify hook claude-stop
bin/ai-task-notify hook claude-permission
bin/ai-task-notify hook claude-notification
bin/ai-task-notify config --feishu-webhook <url> [--feishu-secret <secret>]
bin/ai-task-notify config --show
bin/ai-task-notify config --clear-feishu
```

## 开发

```bash
pnpm install
pnpm build                       # tsc
pnpm --filter @ai-task-notify/server build:bundle   # 单文件 bundle.cjs（App 用）
bash macos/build.sh              # 构建 macOS App（输出 macos/build/AI Task Notify.app）
```

## 目录结构

```
server/           Node + TypeScript：HTTP / WebSocket / Token / mDNS / 飞书 / CLI
macos/            macOS 菜单栏 App 源码 + 构建脚本
integrations/     Codex、Claude Hook 接入说明
docs/             架构、协议、飞书、接入文档
```

## 文档

- [架构](docs/architecture.md)
- [协议](docs/protocol.md)
- [飞书](docs/feishu.md)
- [Codex 接入](docs/codex.md)
- [Claude 接入](docs/claude.md)
- [Android App（暂缓）](docs/android.md)

## 阶段

- Phase 1 ✅ Server（HTTP + WebSocket + Token + mDNS + CLI）
- Phase 4 ✅ Codex Hook 接入（Stop / PermissionRequest / SubagentStop，首次运行需在 `/hooks` 信任）
- Phase 5 ✅ Claude Code Hook 接入（Stop / Notification / PermissionRequest）
- ✅ macOS 菜单栏 App（双击启动，常驻菜单栏）
- ✅ 飞书机器人通知
- Phase 2/3 ⏸️ Android App 与手机/WATCH 5 —— 用户暂缓
