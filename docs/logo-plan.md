# AI Task Notify Logo 需求与执行计划

> 本文档供执行 agent 使用：**完整实现一份全新的品牌 Logo 与全套图标**，替换项目中的占位图标。
> **硬性要求：不得查看、参考、复用项目中现有的任何 logo / 图标图片文件（包括 `desktop/icon-src.png`、`desktop/src-tauri/icons/` 下所有文件）。设计必须从零开始。**

---

## 1. 项目背景

- **产品**：AI Task Notify —— 运行在 macOS 上的轻量级菜单栏 App。
- **核心功能**：接收 Codex CLI / Claude Code 的 Hook 事件（任务开始/结束、权限请求、通知），统一转发并推送通知到飞书机器人单聊（手机 / 手表可同步收到）。
- **产品形态**：菜单栏常驻小工具；主界面是一个小控制面板（状态 / 配置 / 最近事件）。
- **技术栈**：Tauri v2（React + Rust），内嵌 Rust Server；产物为 macOS `.app` / `.dmg`。
- **现状**：应用图标为脚本生成的占位图，需要被真实设计替换。

## 2. 品牌定位与关键词

一句话定位：**「AI Agent 的状态，安静地送到你手上」** —— 常驻、轻量、可靠、不打扰。

设计关键词（按优先级）：

| 关键词 | 含义 |
| --- | --- |
| 通知 / 铃铛 | 产品核心动作：告知、提醒 |
| 状态 / 指示 | 表达「运行中 / 完成 / 等待关注」 |
| 闪电 / 即时 | 毫秒级推送、实时 |
| AI / Agent | 面向 AI 编程代理，带一点科技感 |
| 轻量 / 克制 | 菜单栏工具，不喧宾夺主 |
| 原生 / 现代 | 贴合 macOS Big Sur+ 原生设计语言 |

品牌气质：**现代、极简、扁平、克制、可信赖**。避免幼稚、花哨、厚重、拟物。

## 3. 设计方向（建议，执行 agent 可微调但需符合第 2 节气质）

1. **主图形**：一个简洁、单一、有辨识度的符号，建议从「铃铛 / 闪电 / 状态指示点」中组合或演绎，例如：铃铛 + 状态点，或闪电 + 铃铛。
2. **语义表达**：图标在 16px 菜单栏尺寸下仍能一眼辨认；能自然延伸出「有通知 / 等待关注」的状态感。
3. **配色**：
   - 提供 1 套主色 + 1 个强调色（状态色），建议给出品牌色值（HEX）。
   - 可参考但不必沿用项目现状颜色；设计需自洽。
4. **菜单栏变体**：同一符号需有黑白单色 template 版本（macOS 菜单栏明/暗模式自适应）。
5. **文字**：**App 主图标内不得出现任何文字**（中英文都不行）；文档用横向 wordmark 可另做。

## 4. 使用场景与交付物

### A. 主图标（App Icon，必须）
- 规格：**1024×1024 PNG，RGBA 透明背景**；采用 macOS 风格圆角方形（squircle），主体内容与边缘留约 10% 安全边距。
- 同时提供 **SVG 矢量源文件**（作为设计母版，便于后续修改与派生）。
- 落盘位置：`desktop/icon-src.png`（替换现有占位图）。

### B. Tauri 全套图标（必须）
- 由 `pnpm tauri icon` 从 1024 源图自动生成全套，覆盖：
  - `desktop/src-tauri/icons/icon.icns`、`icon.ico`、`icon.png`
  - `32x32.png`、`64x64.png`、`128x128.png`、`128x128@2x.png`
  - `Square*.png`（Windows Store 各尺寸）、`StoreLogo.png`
  - `android/`、`ios/` 目录下的衍生图标
- 落盘位置：`desktop/src-tauri/icons/`。

### C. 菜单栏 Tray 图标（必须）
- 规格：macOS 菜单栏 **template 单色图标**（黑色 + alpha，系统自动适配明/暗），提供 `18×18` 与 `36×36 (@2x)`（或 16/22pt 规格，由 agent 按 Tauri/macOS 惯例定）。
- **接入要求**：当前 `desktop/src-tauri/src/lib.rs` 用 `default_window_icon()` 作为 tray icon（彩色 app 图标在菜单栏观感差）。执行 agent 需新增/接入专用 template 图标并更新 `lib.rs` 的 tray icon 加载逻辑，保证菜单栏明/暗模式下都清晰。
- 落盘位置：建议 `desktop/src-tauri/icons/` 或 `desktop/src-tauri/assets/`（agent 自定，需在交付说明中写明）。

### D. 飞书机器人头像（可选，推荐）
- 规格：**512×512 PNG**（飞书开放平台机器人头像要求），图形主体须在小尺寸下清晰。
- 说明：此文件**不进仓库**，由用户手动在飞书开放平台上传；agent 产出到项目内（如 `docs/assets/` 或临时目录）并在交付说明中给出路径即可。

### E. Wordmark / 文档 Logo（可选）
- 横向组合：图形符号 + "AI Task Notify" 文字，输出 SVG + PNG（透明背景），用于 README / docs 展示。
- 落盘位置：建议 `docs/assets/`。

## 5. 执行步骤（给执行 agent）

1. **先了解产品**：阅读 `README.md`、`docs/architecture.md`、`docs/feishu.md` 了解产品定位与用法。
   - ⚠️ 不要打开/查看任何现有图标图片文件。
2. **设计主图标**：先做概念（可先在文档/聊天里给 1–3 个方向草图），定稿后产出 SVG 源文件 + 1024×1024 PNG（透明背景）。
3. **替换主图标**：将 PNG 写入 `desktop/icon-src.png`。
4. **重新生成 Tauri 全套图标**：在 `desktop/` 目录运行 `pnpm tauri icon`（源图默认取 `desktop/icon-src.png`），确认 `desktop/src-tauri/icons/` 全部更新。
5. **接入菜单栏 template 图标**：生成单色 template 图标文件，修改 `desktop/src-tauri/src/lib.rs` 的 tray icon 逻辑，使菜单栏使用专用 template 图标。
6. **（可选）飞书头像与 wordmark**：按第 4 节 D / E 产出。
7. **验证**：
   - `cd desktop && pnpm tauri build` 构建成功。
   - 运行 App，检查：菜单栏图标在明/暗模式下均清晰；Finder 中 `.app` 图标正常显示。
8. **提交与交付**：说明所有改动文件路径、设计思路简述，并附最终渲染预览图（如 512px 放大图、明/暗菜单栏效果）。

## 6. 明确不做（范围限制）

- ❌ 不查看 / 不参考 / 不复用现有占位图标设计。
- ❌ 不引入第三方版权素材，不模仿任何知名品牌 logo，必须原创。
- ❌ 不使用照片、复杂渐变、高细节纹理（小尺寸不可读）。
- ❌ App 主图标内不放文字。
- ❌ 不设计 Android App 图标 / Watch 表盘（当前阶段不需要）。
- ❌ 不修改产品名、包标识符、版本号、窗口尺寸等无关配置。

## 7. 验收标准（Checklist）

- [ ] 提供 SVG 矢量源文件（设计母版）。
- [ ] 提供 1024×1024 PNG（透明背景、圆角方形、安全边距）并已写入 `desktop/icon-src.png`。
- [ ] `desktop/src-tauri/icons/` 全套图标已重新生成（icns / ico / png / 各尺寸 / android / ios）。
- [ ] 菜单栏 template 图标已实现并接入 `lib.rs`，明/暗模式下清晰可辨。
- [ ] `pnpm tauri build` 通过，`.app` 图标正常。
- [ ] 交付说明含：改动文件清单、设计思路、预览图。
- [ ] （可选）飞书头像 512×512 PNG、Wordmark（SVG + PNG）。
