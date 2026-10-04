# 协议

## 统一事件模型

```ts
interface AgentEvent {
  id: string;                       // 服务端生成 evt_xxx
  agent: "codex" | "claude";
  event: "task_completed" | "permission_required" | "notification" | "subagent_completed";
  status: "success" | "waiting" | "info";
  project?: string;
  cwd?: string;
  branch?: string;
  title: string;
  message?: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}
```

## HTTP API

### GET /health

```json
{ "status": "ok", "version": "0.1.0" }
```

### POST /api/events

请求头：`Authorization: Bearer <token>`

```json
{
  "agent": "codex",
  "event": "task_completed",
  "status": "success",
  "project": "ddc-fe-admin",
  "title": "Codex 任务完成",
  "message": "菜单重构完成"
}
```

响应：

```json
{ "success": true, "eventId": "evt_xxx" }
```

错误：`401`（无/错 Token）、`400`（JSON 或字段非法）、`404`。

## WebSocket

```
ws://<mac-主机名.local>:3210/ws?token=<token>
```

### 服务端 → 客户端

连接成功：

```json
{ "type": "connected", "serverVersion": "0.1.0" }
```

事件广播：

```json
{
  "type": "agent_event",
  "payload": { "id": "evt_xxx", "agent": "codex", "event": "task_completed", "status": "success", "project": "ddc-fe-admin", "title": "Codex 任务完成", "message": "菜单重构完成", "timestamp": 1791111111111 }
}
```

心跳响应：

```json
{ "type": "pong" }
```

### 客户端 → 服务端

心跳：

```json
{ "type": "ping" }
```

### 重连策略（Android）

指数退避：1s → 2s → 4s → 8s → … 最大 30s；连接成功后重置计数。

## mDNS

- Server 用 `dns-sd -R "AI Task Notify" _ai-task-notify._tcp local <port>` 注册 Bonjour 服务。
- SRV 指向系统 mDNS 主机名（如 `weichaoyingdeMac-mini.local`），不额外广播 A 记录，避免与 macOS 自身记录冲突。
- Android 用 NsdManager 浏览 `_ai-task-notify._tcp` 即可发现 Mac。
