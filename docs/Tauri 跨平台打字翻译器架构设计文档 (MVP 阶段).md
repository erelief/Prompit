# Tauri 跨平台打字翻译器架构设计文档 (MVP 阶段)

## 1. 架构概览与核心范式
本项目旨在将原有的 AHK 脚本重构为基于 Tauri 的跨平台桌面应用。MVP（最小可行性产品）阶段的核心交互范式聚焦于：“全局快捷键唤起独立悬浮窗 -> 输入文本 -> 翻译并自动注入”。

### 1.1 系统流转机制 (Core Flow)

1. **唤醒 (Trigger)**: 用户在任意软件中按下 `Global Shortcut (全局快捷键)`，如 `Alt+Y`。
2. **显示 (Display)**: Tauri 调出居中、无边框、置顶的 Webview 窗口，并自动抢占系统焦点。
3. **处理 (Process)**: 用户在 Web 前端输入待翻译文本，按 Enter 提交，前端调用 LLM API 或本地 AIFW 服务。
4. **注入 (Inject)**: 获得结果后，隐藏 Tauri 窗口，焦点自动交还给原目标窗口，Rust 后端模拟键盘事件（如 `Ctrl+V` / `Cmd+V`）将文本粘贴至目标窗口。
5. **配置 (Configuration)**: 提供独立的图形化设置窗口，供用户填写 API Key、选择模型、配置 AIFW 路径等。

## 2. 技术栈选型 (Tech Stack)

- **核心框架**: Tauri 2.0 (或 1.x)
- **后端 (Rust)**:
  - `tauri-plugin-global-shortcut`: 注册和监听全局快捷键。
  - `enigo` (或 `rdev`): 模拟键盘按键（实现文本注入）。
  - `tauri-plugin-autostart`: 处理开机自启（可选）。
- **前端 (Web)**:
  - 框架: Vue 3 / React
  - 样式: TailwindCSS
  - 请求: 原生 `fetch` 或 `axios`。

## 3. 核心模块与职责划分

### 3.1 Rust 后端模块 (Backend)
负责操作系统底层交互与进程管理。

- **Window Management (窗口管理)**:
  - **主窗口 (悬浮输入框)**：配置 `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`。暴露 `hide_window` 和 `show_window` 接口。
  - **设置窗口 (Settings GUI)**：标准带边框窗口，供用户进行详细配置。可通过系统托盘 (System Tray) 或主窗口上的齿轮图标唤出。
- **Global Shortcut (全局快捷键)**:
  - 仅注册核心快捷键（如 `Alt+Y`）。触发时，通知前端并强制显示主窗口获取焦点。
- **Input Simulation (输入模拟)**:
  - 提供 `invoke` 命令：`paste_text(text: String)`。
  - 逻辑：备份当前剪贴板 -> 写入新文本 -> 模拟按下 `Ctrl+V` (Win/Linux) 或 `Cmd+V` (macOS) -> 恢复剪贴板原内容。
- **AIFW Process Controller (AIFW 进程控制器)**:
  - 负责 `aifw_server.exe` 的生命周期管理。

### 3.2 Web 前端模块 (Frontend)
接管核心业务逻辑，分为两个主要视图 (Views)。

- **View 1: Floating Input (悬浮输入框 UI)**:
  - 极简输入输出界面，响应回车提交。
  - 提供一个细微的“设置”图标，用于打开 View 2。
- **View 2: Settings Panel (图形化设置面板 UI)**:
  - 表单页面，包含：LLM API Key 输入框、模型选择下拉菜单、AIFW 隐私模式全局开关、目标语言选择等。
  - 负责将配置序列化保存至本地系统 (通过 Tauri 的 `fs` API 保存至应用配置目录)。
- **Network & LLM Client (网络与 LLM 客户端)**:
  - 读取本地保存的配置信息。
  - 封装 HTTP 请求，根据隐私模式开关，智能路由至直连 LLM API 或本地 `http://localhost:8844/api/call`。

## 4. Inter-Process Communication (进程间通信) 接口定义

### 4.1 Rust -> Frontend (Events)

- `shortcut_triggered`: 前端收到后清空输入框内容，准备接收新输入。

### 4.2 Frontend -> Rust (Commands)

- `invoke('simulate_paste', { text: "..." })`: 前端翻译完成后调用。
- `invoke('hide_main_window')`: 隐藏悬浮窗。
- `invoke('open_settings_window')`: 打开独立的设置图形界面。

## 5. Agent 实施阶段规划 (Implementation Steps)

- **Phase 1: 基础设施与 UI 搭建**
  - 初始化 Tauri 项目。
  - 编写悬浮输入框前端页面和设置面板前端页面。
  - 实现前端配置的本地读写保存。
- **Phase 2: 操作系统底层打通**
  - 在 Rust 侧注册全局快捷键，实现主窗口的呼出与隐藏。
  - 引入 `enigo`，跑通“前端传值 -> Rust 写入剪贴板 -> 模拟粘贴”的注入链路。
- **Phase 3: 核心业务接入**
  - 编写大模型 API 调用逻辑，实现真正的文本翻译。
- **Phase 4: AIFW 融合**
  - 增加 AIFW 的子进程拉起逻辑。
  - 在前端网络层增加请求拦截，支持向本地 8844 端口转发请求。

## 6. 后续计划 (Future Roadmap)
*(不在 MVP 阶段实现，预留后续通过 GUI 或快捷键扩展的空间)*

- **Dictionary Mode (电子词典模式)**: 独立的翻译排版视图和专用快捷键。
- **Persona Configuration (性格/风格配置)**: 在设置面板中增加 System Prompt 的自定义选项。
- **Slash Commands (斜杠指令)**: 在输入框内解析 `/help`, `/status` 等快速命令。
