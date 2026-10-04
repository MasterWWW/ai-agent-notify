# 飞书通知（自建应用机器人）

推荐使用**飞书开放平台的企业自建应用**里的机器人（App ID + App Secret 方式）。
可以**直接发到你和机器人的单聊窗口**（不用进群），也可以发到群。

## 一、创建自建应用机器人（一次性准备）

1. 打开飞书开放平台：https://open.feishu.cn → **开发者后台** → **创建企业自建应用**。
2. 进入应用 → **添加应用能力** → **机器人**。
3. **权限管理**，开通并**发布版本**：
   - `im:message`（获取与发送单聊、群组消息）
   - `im:message:send_as_bot`（以应用身份发消息）
   - `contact:user.id:readonly`（按手机号查你的 open_id，用于发到单聊）
   - `im:chat:readonly`（可选：列出机器人所在的群/会话）
4. **凭证与基础信息**里记下 **App ID**（`cli_xxx`）和 **App Secret**。
5. 在飞书里**搜索你的机器人，打开和它的单聊**，随便发一句话（比如"你好"）。

## 二、配置（App 弹窗，推荐）

1. 菜单栏 🔔 → 接入方式选「自建应用机器人」。
2. 填 **App ID**、**App Secret**。
3. **单聊方式（推荐，不用建群）**：
   - 填**我的手机号**（需在组织通讯录内）
   - 点 **「查我的单聊」** → 成功后会自动保存 open_id，消息将发到你和机器人的单聊窗口
4. 或 **群聊方式**：点「查群列表」选一个群，填 **群 Chat ID**。
5. 点 **保存** → 点 **测试发送**，单聊/群里立即收到即成功。

## 三、CLI 等价命令

```bash
# 填基础信息
bin/ai-task-notify config --feishu-app-id cli_xxx --feishu-app-secret xxxx

# 单聊方式：按手机号查 open_id 并保存
bin/ai-task-notify feishu me --mobile 138xxxxxxxx

# 群聊方式：列出机器人所在的群/会话
bin/ai-task-notify feishu chats

# 测试发送
bin/ai-task-notify feishu test

# 查看/清除
bin/ai-task-notify config --show
bin/ai-task-notify config --clear-feishu
```

## 四、权限缺失时的报错

查询手机号 open_id 若提示 `Access denied ... [contact:user.id:readonly]`，
按提示链接在开放平台申请该权限并**发布新版本**后再试。

## 五、Webhook 机器人（备选）

```bash
bin/ai-task-notify config --feishu-webhook "https://open.feishu.cn/open-apis/bot/v2/hook/xxx" [--feishu-secret "密钥"]
```

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
