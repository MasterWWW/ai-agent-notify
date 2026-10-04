# Claude Code 接入（Phase 5）

本机安装：`@anthropic-ai/claude-code@2.1.287`（npm 全局，nvmd Node 22.16.0）。

> ⚠️ 当前 `claude` 命令报 `native binary not installed`：postinstall 未执行。接入前先修复：
> ```bash
> cd ~/.nvmd/versions/22.16.0/lib/node_modules/@anthropic-ai/claude-code
> node install.cjs
> ```

## 配置位置

`~/.claude/settings.json`，新增：

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

## 操作步骤（必须备份）

```bash
cp ~/.claude/settings.json ~/.claude/settings.json.bak.ai-task-notify
# 编辑 ~/.claude/settings.json
```

## 回滚

```bash
mv ~/.claude/settings.json.bak.ai-task-notify ~/.claude/settings.json
```

## 事件映射

| Claude Hook | 统一事件 | 状态 |
|---|---|---|
| Stop | task_completed | success |
| PermissionRequest | permission_required | waiting |
| Notification | notification | info（做去重过滤） |

## 验证

```bash
echo '{"hook_event_name":"Stop","cwd":"/Users/weichaoying/code/ddc-order-biz"}' | bin/ai-task-notify hook claude-stop
```
