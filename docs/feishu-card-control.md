# 计划书：Codex / Claude 交互通过飞书交互卡片远程回复

版本：v0.5（Phase C 已应用）
日期：2026-10-05
状态：Phase A/B/C/D 全部实施并验证接线；剩余：用户点击验收（Claude 验收用户暂缓）

---

## 一、结论

**可行。** Codex 和 Claude Code 的「权限请求」类交互，可以通过飞书交互卡片（按钮）远程回复，
用户不用守在终端前。

技术依据（均已核对当前安装版本与官方文档）：

| 能力 | Codex CLI 0.159 | Claude Code 2.1.287 | 飞书 |
|---|---|---|---|
| 权限请求事件 | `PermissionRequest` Hook ✅ | `PermissionRequest` Hook ✅ | — |
| Hook 返回决定 | stdout JSON `decision.behavior: allow/deny` ✅ | stdout JSON `decision.behavior: allow/deny` ✅ | — |
| 等待用户回复 | 默认超时 **600s**，可配 ✅ | 默认超时 **600s**，可配 ✅ | — |
| 无回复兜底 | 走终端正常审批流 ✅ | 走终端正常审批流 ✅ | — |
| 交互卡片发送 | — | — | `channel.send({card})` ✅ |
| 按钮点击回调 | — | — | `card.action.trigger` **走长连接**，无需公网回调地址 ✅ |

> 关键点：两侧 Hook 都支持「阻塞等待 + 返回 allow/deny」；飞书卡片按钮回调
> （`card.action.trigger`）走我们已经建立的长连接（WebSocket），不需要公网 URL / 域名 / HTTPS。

---

## 〇、实施进度

- ✅ **Phase A（服务端 + 卡片闭环）已完成并验证**：
  - `domain/permission.ts`：PermissionRequest 模型 + 内存 Store（TTL 10min + 长轮询唤醒）
  - `transport/server.ts`：`POST /api/permission-requests` + `GET /api/permission-requests/:id/wait`（Bearer 鉴权）
  - `features/feishu/card.ts`：交互卡片（允许/拒绝按钮，value 带 rid+act）
  - `features/permission.ts`：PermissionService（发卡片 / card.action.trigger 决定 / 文本兜底「允许/拒绝 <id>」）
  - `commands/hook.ts`：`hook codex-permission --wait` / `claude-permission --wait` 阻断式等待
  - 验证结果：T1 超时、T2 本人允许→allowed、T3 非本人忽略、T4 文本拒绝、H1/H2 Hook 全链路 allow/deny JSON、H3 Server 不可达静默兜底 —— **全部 PASS**
- ✅ **Phase B（Claude）配置已应用并验证**：
  - `~/.claude/settings.json` 的 PermissionRequest hook 已切换为 `... hook claude-permission --wait` + `"timeout": 570`（备份：`settings.json.bak.ai-task-notify.20261005`）
  - 真实 Claude 无头任务触发验证：日志 `permission card sent requestId=... agent=claude tool=Bash`，Claude 进程被阻塞等待决定
  - ⏳ 待用户在自己终端跑 Claude 触发权限并**点击飞书卡片**完成最终 E2E
- ✅ **Phase C（Codex）已应用并验证**（用户已确认风险 §8.6）：
  - `~/.codex/config.toml`：`approval_policy = "on-request"`；PermissionRequest hook 已加 `--wait` 和 `timeout = 570`（备份：`config.toml.bak.ai-task-notify.20261005`）
  - 接线验证：用真实 Codex PermissionRequest 载荷调用运行中 App → `permission card sent agent=codex`（飞书收到 Codex 卡片）
  - 说明：on-request 下由模型决定何时询问；无头 `codex exec` 实测中模型未触发审批（自行处理了命令被拒），交互终端中模型询问时会走卡片（见 §八.1）
- ✅ **验收脚本已入库**：`scripts/verify-permission.mjs`（`pnpm test:permission`），覆盖验收清单非交互项 S1-S8 / H1 / H3 / D1-D2，全部 PASS
- ✅ **Phase D（安全加固）已实施**：
  - 安全命令自动放行：`git status/diff/log/show/branch/remote/tag`、`ls/pwd/whoami/date/uname/uptime/which`、`echo …` 不发卡片直接 allow（`AI_TASK_NOTIFY_AUTO_ALLOW=0` 关闭）；写操作/Edit/危险命令仍走卡片
  - 超时卡片状态化：请求 TTL 超时后，飞书卡片自动更新为「⏰ 已超时，请在终端处理」（灰色，无按钮）
  - 验证：D1 安全命令 allow JSON、D2 rm/Edit 不自动放行且静默兜底、D3 TTL 超时回调 —— 全部 PASS

---

## 二、目标与边界

### 目标
- 用户不在终端旁时，Codex / Claude 需要权限（执行命令、写文件等）时，
  通过飞书卡片展示请求，用户点【允许】/【拒绝】即可控制。
- 决定后卡片即时更新状态；Agent 继续或终止。

### 第一版不做
- ❌ 不注入任意文本给 Agent（只做「允许 / 拒绝」二元决策）
- ❌ 不支持多用户（只允许绑定的本人 open_id 操作）
- ❌ 不做复杂策略引擎（先做安全命令自动放行白名单）
- ❌ 不改动 Agent 的会话历史 / 输入（Claude 的 `updatedInput` 后续再考虑）

---

## 三、总体链路

```
Codex / Claude（终端会话）
        │ 需要权限
        ▼
   Hook 进程（bin/ai-task-notify hook <agent>-permission）
        │ 1. 读 stdin 事件 JSON
        │ 2. POST /api/permission-requests（本地 Server，带 token）
        │ 3. 轮询等待决定（最长 ~570s）
        ▼
   AI Task Notify Server（Mac，端口 3210）
        │ 4. 渲染交互卡片 → 飞书单聊
        │ 5. 收到 card.action.trigger（长连接）→ 校验身份 → 记录决定
        ▼
   飞书 App（手机/手表）→ 用户点【允许】/【拒绝】
        ▲
        └── 决定回流：Hook 拿到结果 → stdout 输出 JSON → Agent 继续/拦截
```

---

## 四、关键机制（已核实）

### 4.1 Codex：PermissionRequest Hook 可返回决定
官方文档（developers.openai.com/codex/hooks）：
- `PermissionRequest` 在 Codex 需要权限时触发，matcher 可按工具名过滤（Bash / apply_patch / MCP）。
- Hook 在 **stdout** 输出 JSON 即返回决定：

```json
{ "hookSpecificOutput": { "hookEventName": "PermissionRequest",
    "decision": { "behavior": "allow" } } }

{ "hookSpecificOutput": { "hookEventName": "PermissionRequest",
    "decision": { "behavior": "deny", "message": "你在飞书拒绝了" } } }
```

- 多个 Hook 决定时，任意 deny 优先；没有决定则走终端正常审批流（我们的兜底）。
- 超时：命令型 Hook 默认 **600s**，可配置（SessionEnd / Interrupt 除外）。
- 注意：当前 `~/.codex/config.toml` 是 `approval_policy = "never"`（从不询问），
  需改为 `on-request`（模型决定何时询问），Hook 才会触发。

### 4.2 Claude Code：PermissionRequest Hook 可返回决定
官方文档（code.claude.com/docs/en/hooks）：
- `PermissionRequest` 仅在 Claude「即将向你询问权限」或「本该自动拒绝」时触发。
- 输出格式：

```json
{ "hookSpecificOutput": { "hookEventName": "PermissionRequest",
    "decision": { "behavior": "allow" } } }
```

- `allow` 时可选 `updatedInput` 改写工具入参、`updatedPermissions` 记住规则（第一版不用）。
- 超时：命令型 Hook 默认 **600s**。
- 注意：当前 `~/.claude/settings.json` 已配置 `PermissionRequest` Hook（通知模式），
  第二版改成「等待决定模式」即可，无需改事件名。

### 4.3 飞书：卡片按钮回调走长连接
- SDK（@larksuiteoapi/node-sdk 1.74）在长连接 Dispatcher 中注册了 `card.action.trigger`，
  与 `im.message.receive_v1` 同一 WebSocket 通道 → **无需公网回调地址**。
- 卡片回调事件模型：`{ messageId, chatId, operator.openId, action.value }`，
  `action.value` 携带自定义数据（requestId + 允许/拒绝）。
- 发送卡片：`channel.send(chatId, { card })`；更新卡片：`channel.updateCard(messageId, card)`。

---

## 五、架构设计（沿用现有分层）

```
server/src/
├── domain/
│   └── permission.ts        PermissionRequest / PermissionDecision 模型 + 内存 Store（TTL）
├── transport/
│   └── server.ts            + POST /api/permission-requests
│                            + GET  /api/permission-requests/:id/wait（长轮询）
├── features/feishu/
│   ├── card.ts              交互卡片构建（标题/命令/按钮/value）
│   └── connection.ts        on("cardAction") → resolvePermissionRequest(...)
└── commands/hook.ts         codex-permission / claude-permission 改「等待决定」模式
```

### 5.1 领域模型
```ts
interface PermissionRequest {
  id: string;                 // 如 pr_<random>
  agent: "codex" | "claude";
  toolName: string;           // Bash / apply_patch / ...
  command?: string;           // 具体命令
  cwd?: string;
  project?: string;           // 从 cwd 推断
  status: "pending" | "allowed" | "denied" | "timeout";
  createdAt: number;
  decidedAt?: number;
  decision?: "allow" | "deny";
}
```

### 5.2 本地 HTTP API（沿用现有 Bearer Token 鉴权）
- `POST /api/permission-requests`
  入参 `{ agent, toolName, command, cwd }` → 返回 `{ requestId }`；同时发飞书卡片。
- `GET /api/permission-requests/:id/wait?timeout=550000`
  长轮询：决定后立刻返回 `{ status, decision }`；超时返回 `{ status: "timeout" }`。
- 幂等 / 清理：`pending` 超过 10 分钟自动标记 `timeout`，Store 定时清理。

### 5.3 飞书卡片
```
┌──────────────────────────────────┐
│ ⚠️ Codex 请求执行命令             │
│                                  │
│ 项目：ai-agent-notify             │
│ 目录：~/Documents/ChatGPT/...     │
│                                  │
│ pnpm add some-package            │
│                                  │
│ [ 允许 ]        [ 拒绝 ]          │
└──────────────────────────────────┘
```
- 按钮 `value = { rid: "<requestId>", act: "allow" | "deny" }`。
- 点击后：校验 `operator.openId === 绑定的 open_id`（不是本人则忽略）；
  记录决定；`updateCard` 把按钮替换为「✅ 已允许 / ⛔ 已拒绝」。
- 兜底：如果卡片回调异常，用户可回复文本 `允许 <id>` / `拒绝 <id>`（走现有消息通道）。

### 5.4 阻断式 Hook
- 命令：`ai-task-notify hook codex-permission` / `claude-permission`（现有入口，增加 `--wait` 语义）。
- 流程：读 stdin JSON → 提取 `tool_name / tool_input.command / cwd / session_id`
  → POST 到本地 Server → 长轮询等待 → 超时则 **exit 0 且不输出决定**（Agent 走终端审批流兜底）。
- **失败永不阻断 Agent**：Server 连不上 → 立即 exit 0 无决定；任何异常都不阻塞。

### 5.5 配置变更（需用户同意）
- Codex `~/.codex/config.toml`：
  ```toml
  approval_policy = "on-request"        # 原 "never"
  [[hooks.PermissionRequest]]
  [[hooks.PermissionRequest.hooks]]
  type = "command"
  command = ".../bin/ai-task-notify hook codex-permission"
  timeout = 570                          # 留余量，默认 600
  ```
- Claude `~/.claude/settings.json`：把现有 PermissionRequest hook 的 command 换成等待模式，
  加 `"timeout": 570`。

---

## 六、分阶段实施

### Phase A：服务端（无 Agent 参与，可独立验收）
1. 领域模型 + 内存 Store（TTL 10min，定时清理）。
2. HTTP API：POST 创建 + 长轮询等待（带 token）。
3. 飞书卡片构建 + `card.action.trigger` 处理（校验本人 open_id → 决定 → updateCard）。
4. 验收：`curl` 造一个 request → 收到卡片 → 点【允许】→ 长轮询返回 allow。

### Phase B：Claude 接入（先做简单、风险低的一侧）
1. `hook claude-permission` 改等待模式。
2. settings.json 配置 timeout。
3. 验收：终端跑 `claude`，触发权限请求（如 `rm` / 安装依赖）→ 飞书卡片 →
   点允许 → 命令执行；点拒绝 → 命令被拦截；不点 → 570s 后终端出现审批。

### Phase C：Codex 接入
1. `hook codex-permission` 改等待模式。
2. config.toml 改 `approval_policy = "on-request"` + timeout。
3. 验收：跑 `codex`，模型决定需要审批时 → 飞书卡片 → 允许/拒绝。

### Phase D：体验与安全加固
1. 安全命令自动放行白名单（git status / ls / cat / pwd 等不发卡片，直接 allow）——
   减少打扰（可先用 PreToolUse 或 Hook 内策略）。
2. 卡片按钮点击后禁用 / 状态化；超时卡片显示「已超时，请在终端处理」。
3. 异常测试：Server 挂掉 / 飞书 API 失败 / 卡片被他人点击 → Agent 不受影响。

---

## 七、验收清单（最终）

- [ ] Claude 权限请求 → 飞书卡片 → 允许/拒绝生效
- [ ] Codex 权限请求 → 飞书卡片 → 允许/拒绝生效
- [ ] 不在终端回复，Agent 等待期间不中断
- [ ] 不点按钮 → 超时回退终端审批流，Agent 不卡死
- [ ] 非本人点击卡片被忽略
- [ ] Server / 飞书异常 → Hook exit 0，Agent 任务不受影响

---

## 八、风险与限制

1. **Codex `on-request` 语义**：由模型决定何时询问，可能过多或过少。
   解决：Phase D 加命令白名单 / 策略；必要时改 PreToolUse 强制策略。
2. **600s 超时**：用户需在 10 分钟内回复，否则回退终端。可后续用「卡片持续有效 + 结果异步」改进。
3. **卡片回调走长连接**：需在开放平台确认卡片回调方式选「长连接」；
   若个别租户不支持，回退方案=文本命令（`允许 <id>`）。
4. **多会话并发**：多个 Agent 会话同时请求时，卡片需带 session/request 上下文区分（requestId 已覆盖）。
5. **Codex 版本**：PermissionRequest 决定能力依赖较新版本（本机 0.159 alpha 已支持）；
   升级/降级后需回归。
6. **本机 Codex 全自动环境**：当前 Codex（ChatGPT App 内）是 `approval_policy=never`，
   改成 on-request 会影响这台机器上所有 Codex 会话；建议先在单独终端会话验证，
   或按项目/目录拆分配置。

---

## 九、开放平台需确认/配置

1. 事件订阅：`im.message.receive_v1`（已有）。
2. 卡片回调：确认「使用长连接接收卡片回调」（`card.action.trigger`）。
3. 权限：`im:message`（已有，发卡片即发消息）。
4. 可用范围：已含自己（已有）。

---

## 十、下一步（等待确认后开工）

1. 确认本计划书（尤其第八节风险 6：是否接受把本机 Codex 改为 `on-request`）。
2. 先做 Phase A（服务端 + 卡片闭环，不影响任何 Agent）。
3. Phase A 验收通过后，接 Claude（Phase B），再 Codex（Phase C）。
