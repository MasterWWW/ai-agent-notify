# Codex 接入（Phase 4）

本机 Codex CLI：`0.159.0-alpha.12.1`（位于 ChatGPT.app 内）。

支持的 Hook 事件（以当前版本为准）：`Stop`、`PermissionRequest`、`SubagentStop`、`SessionStart`、`SessionEnd`、`PreToolUse`、`PostToolUse` 等。

## 配置位置

`~/.codex/config.toml`，新增：

```toml
[hooks]
Stop = ["/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook codex-stop"]
PermissionRequest = ["/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook codex-permission"]
SubagentStop = ["/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook codex-subagent-stop"]
```

> 说明：`bin/ai-task-notify` 是 repo 根目录的可执行脚本（要求先 `pnpm build`）。
> Hook 命令不传 Token：CLI 会读取 `~/.ai-task-notify/token`（Server 首次启动时自动生成）。

## 操作步骤（必须备份）

```bash
cp ~/.codex/config.toml ~/.codex/config.toml.bak.ai-task-notify
# 编辑 ~/.codex/config.toml 加入 [hooks]
```

## 回滚

```bash
mv ~/.codex/config.toml.bak.ai-task-notify ~/.codex/config.toml
```

## 事件映射

| Codex Hook | 统一事件 | 状态 |
|---|---|---|
| Stop | task_completed | success |
| PermissionRequest | permission_required | waiting |
| SubagentStop | subagent_completed | success |

## 验证

```bash
# 模拟 Codex Stop payload（应能收到通知）
echo '{"hook_event_name":"Stop","cwd":"/Users/weichaoying/code/ddc-fe-admin"}' | bin/ai-task-notify hook codex-stop
```
