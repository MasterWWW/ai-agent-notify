# 飞书通知（自建应用机器人）

推荐使用**飞书开放平台的企业自建应用**里的机器人（App ID + App Secret 方式），
把通知发到你的飞书群/会话。群内「自定义机器人 Webhook」方式作为备选保留。

## 一、创建自建应用机器人（一次性准备）

1. 打开飞书开放平台：https://open.feishu.cn → **开发者后台** → **创建企业自建应用**。
2. 进入应用 → **添加应用能力** → **机器人**（开启后应用才有发消息能力）。
3. **权限管理**，开通以下权限并发布版本：
   - `im:message`（获取与发送单聊、群组消息）
   - `im:message:send_as_bot`（以应用的身份发消息）
   - `im:chat:readonly`（获取群列表，用于查找 Chat ID）
4. **版本管理与发布**：创建版本 → 申请发布（个人可用直接发布到自己的组织；若无法发布，可把应用设为「可用性」里的测试范围，或使用测试企业）。
5. 在 **凭证与基础信息** 里记下：
   - **App ID**（形如 `cli_xxxx`）
   - **App Secret**
6. 在飞书里建一个群（或拉一个只有自己的群），把**机器人加进群**。

## 二、拿到群 Chat ID（二选一）

### 方式 A：App 弹窗「查群列表」
菜单栏 🔔 → 填入 App ID / App Secret → 点**查群列表**，会列出机器人所在的群和 Chat ID，复制即可。

### 方式 B：CLI
```bash
bin/ai-task-notify config --feishu-app-id cli_xxx --feishu-app-secret xxxx
bin/ai-task-notify feishu chats
# 输出：<chat_id>  <群名>，例如 oc_xxxxxxxx  我的通知群
```

## 三、配置（二选一）

### App 弹窗（推荐）
菜单栏 🔔 → 接入方式选「自建应用机器人」→ 填 **App ID / App Secret / 群 Chat ID** → **保存** → **测试发送**。

### CLI
```bash
bin/ai-task-notify config --feishu-app-id cli_xxx --feishu-app-secret xxxx --feishu-chat-id oc_xxx
bin/ai-task-notify feishu test    # 发一条测试消息
bin/ai-task-notify config --show
bin/ai-task-notify config --clear-feishu
```

## 四、Webhook 机器人（备选）

```bash
bin/ai-task-notify config --feishu-webhook "https://open.feishu.cn/open-apis/bot/v2/hook/xxx" [--feishu-secret "密钥"]
```

## 五、消息格式

```
🤖 Codex
ddc-fe-admin
✅ 任务完成
菜单重构完成
```

- success → ✅，waiting → 🟡，info → 💬
- 自建应用方式用 tenant_access_token 调 `im/v1/messages` 发送，token 自动缓存、过期自动刷新。
- 配置保存在 `~/.ai-task-notify/config.json`（0600），不会写入 Git。

## 六、验证

```bash
bin/ai-task-notify feishu test
```

或跑任意 Codex / Claude 任务，飞书群立即收到通知。
