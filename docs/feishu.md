# 飞书通知（自建应用机器人）

推荐使用**飞书开放平台的企业自建应用**里的机器人（App ID + App Secret 方式）。
可以直接发到**你和机器人的单聊窗口**（不用进群），只需要复制单聊的「群 ID」。

## 一、创建自建应用机器人（一次性准备）

1. 打开飞书开放平台：https://open.feishu.cn → **开发者后台** → **创建企业自建应用**。
2. 进入应用 → **添加应用能力** → **机器人**。
3. **权限管理**，开通并**发布版本**：
   - `im:message`（获取与发送单聊、群组消息）
   - `im:message:send_as_bot`（以应用身份发消息）
4. **凭证与基础信息**里记下 **App ID**（`cli_xxx`）和 **App Secret**。
5. （重要）**版本管理与发布** → 可用范围：把**你自己**加入可用范围（否则会报 230013 Bot has NO availability to this user）。
6. 在飞书里**搜索你的机器人，打开和它的单聊**。

## 二、拿单聊的 Chat ID（不用查手机号、不用进群）

在飞书里打开**和机器人的单聊窗口** → 点右上角 **⋯**（更多）→ **群设置** → 找到**群 ID**（形如 `oc_xxx`）→ 复制。

> 官方说明：「群 ID 获取方式支持单聊或群聊两种模式，群成员可通过群设置页面查看群 ID。」

## 三、配置（App 弹窗，推荐）

1. 菜单栏 🔔 → 接入方式选「自建应用机器人」。
2. 填 **App ID**、**App Secret**、**单聊/群 Chat ID**（就是上面复制的 `oc_xxx`）。
3. 点 **保存** → 点 **测试发送**，单聊窗口立即收到。

## 四、CLI 等价命令

```bash
bin/ai-task-notify config --feishu-app-id cli_xxx --feishu-app-secret xxxx --feishu-chat-id oc_xxx
bin/ai-task-notify feishu test
bin/ai-task-notify config --show
bin/ai-task-notify config --clear-feishu
```

## 五、其他方式

- **群聊**：建一个群把机器人拉进去，群设置里复制群 ID，填到 Chat ID 即可。
- **按手机号查 open_id 发单聊（高级，可选）**：需要 `contact:user.id:readonly` 权限，
  `bin/ai-task-notify feishu me --mobile 138xxxxxxxx` 会自动保存 open_id。
- **Webhook 机器人（备选）**：`bin/ai-task-notify config --feishu-webhook <url> [--feishu-secret <密钥>]`

## 六、消息格式

```
🤖 Codex
ddc-fe-admin
✅ 任务完成
菜单重构完成
```

- success → ✅，waiting → 🟡，info → 💬
- 自建应用用 tenant_access_token 调 `im/v1/messages` 发送，token 自动缓存、过期自动刷新。
- 配置保存在 `~/.ai-task-notify/config.json`（0600），不会写入 Git。
