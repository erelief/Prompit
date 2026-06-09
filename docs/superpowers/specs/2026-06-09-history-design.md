# FloatingInput 输入/返回历史记录

## 概述

为 FloatingInput 添加翻译历史记录功能。记录用户输入和 LLM 返回的内容对，支持历史浏览（↑↓ 快捷键）、历史列表查看、加密本地存储、条数配置和一键清除。

## 数据模型

```typescript
interface HistoryEntry {
  input: string;    // 用户输入的 prompt
  output: string;   // LLM 返回的翻译结果
  timestamp: number; // Unix 毫秒时间戳
}
```

- 存储文件：`history.json`（位于 app data dir）
- 加密方式：AES-256-GCM + SHA-256 密钥派生（`hostname:username:com.translator.realtime`），与 `secrets.rs` / `persona.rs` / `dictionary.rs` 完全一致
- 存储格式：`{ ciphertext: base64, nonce: base64 }` JSON
- 最大条数：默认 50，用户可配置；超出时丢弃最旧的记录

## 后端（Rust）

新增 `src-tauri/src/commands/history.rs`：

| 命令 | 签名 | 说明 |
|------|------|------|
| `read_history` | `(app: AppHandle) -> Result<Vec<HistoryEntry>, String>` | 解密读取历史列表 |
| `save_history` | `(app: AppHandle, entries: Vec<HistoryEntry>) -> Result<(), String>` | 清理超出上限条目后加密存储 |
| `clear_history` | `(app: AppHandle) -> Result<(), String>` | 删除 `history.json` |

- 复用 `derive_key()` 模式（从 `secrets.rs` 提取或直接复制）
- 在 `src-tauri/src/commands/mod.rs` 注册模块
- 在 `src-tauri/src/lib.rs` 注册 invoke handlers

## 路由与页面

新增路由：

```typescript
{
  path: "/history",
  name: "history",
  component: () => import("../views/HistoryPanel.vue"),
}
```

### 入口

FloatingInput 发送按钮右侧添加历史按钮（`History` 图标，来自 `@lucide/vue`），点击后：

1. `router.push('/history')`
2. 调用 `useSettingsWindow(560, 380)` 初始化窗口大小（与 Onboarding 一致：380 宽 × 560 高）
3. 窗口 `resizable`，用户可自行调整大小

### HistoryPanel.vue

独立 Vue 页面：

- **顶部**：返回按钮，点击回到 FloatingInput（`router.push('/')`）
- **主体**：历史记录列表，`overflow-y: auto` 滚动，内容自适应窗口大小
- **列表排序**：时间倒序（最新在前）
- **每条记录**占两行：第一行输入文本截断 + `…`，第二行输出文本截断 + `…`；宽度溢出用 `text-overflow: ellipsis` 处理
- **点击某条记录**：恢复到 FloatingInput 的输入/输出框中（`router.push('/')` 并传递数据）
- 列表生长方向遵循项目既有规范（`growAbove` 模式）

## ↑↓ 快捷键（Terminal 体验）

在 FloatingInput 的 `handleKeydown` 中扩展：

| 按键 | 条件 | 行为 |
|------|------|------|
| `↑` | 光标在 textarea 首行首位 | 恢复上一条历史到输入框和输出框，进入"历史浏览"状态 |
| `↓` | 光标在 textarea 末行末位 | 恢复下一条历史 |
| `Enter` | 历史浏览状态中 | 发送当前内容（原始或修改后），产生新历史记录 |
| 任意编辑 / `Esc` | 历史浏览状态中 | 退出历史浏览状态 |

"历史浏览"状态用一个 `historyIndex: ref<number | null>` 追踪，`null` 表示不在浏览状态。

## General Settings 扩展

在 Settings 的 General 区域新增一个分块（section），包含：

1. **历史记录条数**：数字输入框，绑定到 `appConfig.history_limit`，默认 50
2. **清除历史记录**：按钮，点击后弹出二次确认，调用 `clear_history` 命令删除所有历史

## 新增文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/commands/history.rs` | 历史记录的加密读写 Rust 命令 |
| `src/views/HistoryPanel.vue` | 历史记录列表页面 |

## 修改文件

| 文件 | 修改内容 |
|------|----------|
| `src/router/index.ts` | 新增 `/history` 路由 |
| `src/views/FloatingInput.vue` | 添加历史按钮、↑↓ 快捷键逻辑、历史浏览状态管理 |
| `src/views/Settings.vue` | General 区域新增历史记录配置分块 |
| `src/stores/config.ts` | 新增 `history_limit` 配置项 |
| `src-tauri/src/commands/mod.rs` | 注册 history 模块 |
| `src-tauri/src/lib.rs` | 注册 history invoke handlers |
| `src-tauri/src/config.rs` | 新增 `history_limit` 字段及默认值 |

## 前端一致性

使用 `/frontend-design` 技能确保新 UI 组件（历史按钮、列表项、Settings 分块）与现有设计语言保持一致。
