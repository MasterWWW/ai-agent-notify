# 飞书机器人通知

事件到达 Server 后，若配置了飞书 Webhook，会自动推送到飞书群机器人，手机/桌面飞书即可收到通知。

## 创建飞书机器人

1. 打开飞书，进入任意群聊（或新建一个只有自己的群）。
2. 群设置 → 群机器人 → 添加机器人 → **自定义机器人**。
3. 复制 **Webhook 地址**，形如：
   ```
   https://open.feishu.cn/open-apis/bot/v2/hook/xxxx-xxxx
   ```
4. （可选）在安全设置里开启「签名校验」，把**密钥**也记下来。

## 配置方式（二选一）

### 方式 A：macOS App 菜单

菜单栏 AI Task Notify 图标 → **配置飞书…** → 粘贴 Webhook 地址（和可选密钥）→ 保存。

### 方式 B：CLI

```bash
bin/ai-task-notify config --feishu-webhook "https://open.feishu.cn/open-apis/bot/v2/hook/xxx" [--feishu-secret "密钥"]
bin/ai-task-notify config --show          # 查看（密钥会打码）
bin/ai-task-notify config --clear-feishu  # 清除
```

配置保存在 `~/.ai-task-notify/config.json`（0600 权限），不会写入 Git。

## 消息格式

```
🤖 Codex
ddc-fe-admin
✅ 任务完成
菜单重构完成
```

- success → ✅，waiting → 🟡，info → 💬
- 配置了密钥时会带 `timestamp` + `sign`（HMAC-SHA256），飞书校验通过才发送。

## 验证

```bash
bin/ai-task-notify test --message "飞书测试"
```

配置后跑任意 Codex / Claude 任务，或上面的 test，飞书群应立即收到。
