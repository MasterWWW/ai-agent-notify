# Codex 接入

本机 Codex CLI：`0.159.0-alpha.12.1`（位于 ChatGPT.app 内）。

参考：官方文档 [Codex Hooks](https://developers.openai.com/codex/hooks)（已按官方格式核对）。

## 配置位置

`~/.codex/config.toml`（本机已接入，2026-10-04）。

官方当前格式（数组表）：

```toml
[[hooks.Stop]]
[[hooks.Stop.hooks]]
type = "command"
command = "/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook codex-stop"

[[hooks.PermissionRequest]]
[[hooks.PermissionRequest.hooks]]
type = "command"
command = "/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook codex-permission"

[[hooks.SubagentStop]]
[[hooks.SubagentStop.hooks]]
type = "command"
command = "/Users/weichaoying/Documents/ChatGPT/ai-agent-notify/bin/ai-task-notify hook codex-subagent-stop"
```

> 旧格式 `[hooks] Stop = ["..."]` 已过时，不要使用。

## Hook 输入

每个命令 Hook 通过 **stdin** 收到一个 JSON 对象，常用字段：

- `session_id`、`cwd`、`hook_event_name`、`transcript_path`、`model`
- `Stop` / `PermissionRequest` / `SubagentStop` 额外带 `turn_id`、`permission_mode`

CLI 会从中提取 `project`（cwd 目录名）并映射成统一事件。

## 信任机制（重要）

Codex 对非托管 Hook 需要**人工信任**后才执行。首次运行：

```bash
codex
```

看到 "hooks need review" 警告后，在会话里输入 `/hooks`，找到 AI Task Notify 的三条 Hook，选择信任。

一次性验证可跳过信任：

```bash
codex exec --dangerously-bypass-hook-trust "测试提示词"
```

## 备份与回滚

```bash
# 备份（本机已做：config.toml.bak.ai-task-notify.20261004）
cp ~/.codex/config.toml ~/.codex/config.toml.bak.ai-task-notify.$(date +%Y%m%d)

# 回滚
mv ~/.codex/config.toml.bak.ai-task-notify.20261004 ~/.codex/config.toml
```

## 事件映射

| Codex Hook | 统一事件 | 状态 |
|---|---|---|
| Stop | task_completed | success |
| PermissionRequest | permission_required | waiting |
| SubagentStop | subagent_completed | success |

## 验证结果（本机实测）

```text
codex exec ...  ->  hook: Stop  ->  hook: Stop Completed
Server 日志: event=evt_xxx agent=codex event=task_completed status=success project=tmp clients=0
```
