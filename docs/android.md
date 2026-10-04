# Android App（Phase 2，规划）

- 技术栈：Kotlin + Jetpack Compose + OkHttp WebSocket + NotificationManager。
- 首页：Server 地址（默认填 `.local` 主机名）、Token、连接状态（● Connected / ○ Disconnected / ◌ Connecting）、[Connect] 按钮、最近事件列表。
- 自动重连：指数退避 1s→2s→4s→…→30s，连接成功重置。
- 通知 Channel：`AI Task`（默认）+ `AI Permission`（高重要性）。
- 主机名输入：支持 `weichaoyingdeMac-mini.local` 这类 mDNS 名称（系统解析器支持 `.local`）。
- 可选增强：NSD 浏览 `_ai-task-notify._tcp` 自动发现 Mac，用户无需手填。
- 前台常驻：验证 vivo 后台限制情况，必要时用 Foreground Service。
