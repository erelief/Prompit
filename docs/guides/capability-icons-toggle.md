# 模型能力图标开关 (Capability Icons Toggle)

## 打开/关闭的是什么

在模型名称后显示多模态输入能力图标（`ImageUp` 图片、`AudioUp` 音频、`VideoUp` 视频）。当检测到模型支持某种输入模态时，在以下界面显示对应图标：

- FloatingInput 活动模型按钮及下拉菜单
- Settings 翻译/Sparkle 模型选择下拉菜单
- Settings Provider 卡片模型标签
- Onboarding 步骤 3 模型选择

默认关闭。开启后为实验性功能，不影响用户正常使用判断。

## 怎么打开和关闭

编辑 `src-tauri/src/config.rs`，找到 `impl Default for AppConfig` 中的：

```rust
show_capability_icons: false,  // ← 改成 true 开启
```

- `false` = 关闭（默认）
- `true` = 开启

改完后重新构建即可（`npm run tauri dev` 或 `npm run build`）。此字段不写入 config.json（`skip_serializing`），代码中的值是唯一控制源。
