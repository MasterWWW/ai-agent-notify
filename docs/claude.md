# Claude Code 接入

本机安装：`@anthropic-ai/claude-code@2.1.287`（npm 全局，nvmd Node 22.16.0）。

## 修复二进制（已完成，2026-10-04）

之前 `claude` 报 `native binary not installed`（postinstall 未执行）。已修复：

```bash
cd ~/.nvmd/versions/22.16.0/lib/node_modules/@anthropic-ai/claude-code
npm install --registry=https://registry.npmmirror.com --no-save @anthropic-ai/claude-code-darwin-x64
node install.cjs
```

验证：`claude --version` → `2.1.287 (Claude Code)`。

## 配置位置

`~/.claude/settings.json`（本机已接入，2026-10-04，备份 `settings.json.bak.ai-task-notify.20261004`）：

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [{ "type": "command", "command": "/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook claude-stop" }] }
    ],
    "PermissionRequest": [
      { "hooks": [{ "type": "command", "command": "/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook claude-permission" }] }
    ],
    "Notification": [
      { "hooks": [{ "type": "command", "command": "/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook claude-notification" }] }
    ]
  }
}
```

## 备份与回滚

```bash
cp ~/.claude/settings.json ~/.claude/settings.json.bak.ai-task-notify.$(date +%Y%m%d)
mv ~/.claude/settings.json.bak.ai-task-notify.20261004 ~/.claude/settings.json
```

## 事件映射

| Claude Hook | 统一事件 | 状态 |
|---|---|---|
| Stop | task_completed | success |
| PermissionRequest | permission_required | waiting |
| Notification | notification | info |

## 验证结果（本机实测）

```text
claude -p "只回复两个字：收到"
Server 日志: event=evt_xxx agent=claude event=task_completed status=success project=ai-agent-notify clients=0
```

> 注意：`Notification` Hook 会收到很多普通通知，可能需要按消息内容做过滤，避免刷屏。
