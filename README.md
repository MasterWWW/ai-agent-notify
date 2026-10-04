# AI Task Notify

运行在 macOS 上的轻量 AI Agent 任务通知：接收 Codex CLI / Claude Code 的 Hook 事件，推送到**飞书机器人的单聊窗口**（手机/手表可同步收到）。

```
Codex CLI ──Hook──┐
                 ├─▶ AI Task Notify（macOS 菜单栏 App）
Claude Code ─Hook─┘          │
                             ├─▶ 飞书单聊（你直接收通知）✅ 主路径
                             └─▶ WebSocket（手机 App，暂缓）
```

## 一键启动：macOS App（推荐）

```bash
open /Applications/"AI Task Notify.app"
```

- 菜单栏出现 🔔 图标，启动即自动拉起本地 Server（端口 3210）。
- 菜单里可查看：运行状态、主机名（mDNS 本地名称）、Token、最近事件。
- **配置飞书…** 里粘贴 App ID / App Secret（单聊 Chat ID 可选），点「测试发送」验证。
- 数据都在 `~/.ai-task-notify/`（token、config.json、events.jsonl、server.log）。

## 飞书通知（自建应用机器人）

在 [飞书开放平台](https://open.feishu.cn) 创建**企业自建应用**并开启「机器人」能力，拿到 App ID / App Secret，在 App 弹窗里填入并「测试发送」。完整流程见 [docs/feishu.md](docs/feishu.md)。

**发到哪，二选一（都不用查手机号）：**
- ✅ **open_id（默认）**：之前已通过手机号查好并保存，直接发到你和机器人的单聊，无需任何额外操作。
- 可选 **Chat ID**：飞书里和机器人单聊 → 右上角 ⋯ → 群设置 → 复制群 ID（`oc_xxx`）填入，会优先使用。

## 本地名称访问（不用 IP）

手机端 / 客户端填 Mac 的 mDNS 主机名，IP 变了不用改：

```
ws://<主机名>.local:3210/ws?token=xxxx
```

Server 启动时自动打印主机名与局域网 IP，并用 `dns-sd` 注册 Bonjour 服务 `_ai-task-notify._tcp`。

## CLI（可选）

```bash
bin/ai-task-notify server            # 直接跑 Server（不进 App 时）
bin/ai-task-notify status            # 健康检查
bin/ai-task-notify test              # 发一条测试事件（走完整链路）
bin/ai-task-notify feishu test       # 飞书测试发送
bin/ai-task-notify config --show     # 查看配置（密钥打码）
bin/ai-task-notify hook <kind>       # Codex / Claude Hook 转发（失败永不阻断 Agent）
```

## 架构

四层分离：**入口**（CLI 分发）→ **链路**（HTTP/WS/Hook/mDNS）→ **业务**（统一事件 + Pipeline 编排）→ **功能**（飞书通知/历史/配置）+ **基础**（鉴权/日志/状态）。

```
server/src/
├── index.ts                命令分发
├── commands/               server / config / feishu / hook / status / test
├── domain/                 types · normalize · pipeline（事件编排）
├── transport/              server（HTTP+WS）· ws · hooks · client · mdns
├── features/               notifier · history · config
│   └── feishu/             api · channels · render · queries
└── core/                   auth · logger · state · version

macos/
├── ConfigStore.swift       数据层
├── ServerController.swift  进程层
├── AppMenuView.swift       UI 层
└── AITaskNotifyApp.swift   @main 入口
```

详见 [docs/architecture.md](docs/architecture.md)。

## 开发

```bash
pnpm install
pnpm build                       # tsc
pnpm --filter @ai-task-notify/server build:bundle   # 单文件 bundle.cjs（App 用）
bash macos/build.sh              # 构建 macOS App（输出 macos/build/AI Task Notify.app）
```

## 文档

- [架构](docs/architecture.md)
- [飞书](docs/feishu.md)
- [Codex 接入](docs/codex.md)
- [Claude 接入](docs/claude.md)
- [协议](docs/protocol.md)
- [Android App（暂缓）](docs/android.md)

## 阶段

- ✅ Phase 1 Server（HTTP + WebSocket + Token + mDNS + CLI）
- ✅ Phase 4 Codex Hook 接入（Stop / PermissionRequest / SubagentStop）
- ✅ Phase 5 Claude Code Hook 接入（Stop / Notification / PermissionRequest）
- ✅ macOS 菜单栏 App + 飞书机器人通知
- ⏸️ Android App 与手机/WATCH 5 —— 用户暂缓（飞书通知已覆盖）
