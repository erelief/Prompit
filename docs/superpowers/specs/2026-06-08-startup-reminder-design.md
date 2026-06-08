# Startup Shortcut Reminder Screen

**Date:** 2026-06-08
**Status:** Approved

## Summary

Add a small reminder screen that appears at app launch (non-first-run) to show the user the global shortcut key. The screen auto-closes after 10 seconds, or when the user clicks the close button or presses the shortcut. A toggle in Settings > Interface controls whether this screen appears.

## Requirements

1. Non-first-run launch: show a centered card reminding the user of the shortcut key
2. Close button (×) in top-right corner with a 10s countdown; auto-closes when timer reaches 0
3. Pressing the global shortcut also closes the screen
4. Settings > Interface toggle to enable/disable (default: enabled)
5. First-run (providers empty): show Onboarding instead (existing behavior, unchanged)

## Design

### 1. Startup Flow & Router Logic

**Router guard (`src/router/index.ts`):**

```
providers.length === 0                     → /onboarding
providers.length > 0 && show_startup_reminder === true  → /startup-reminder
providers.length > 0 && show_startup_reminder === false → / (floating-input)
```

The `/startup-reminder` route is also guarded: if `providers.length === 0`, redirect to `/onboarding`.

When navigating to `/startup-reminder`, the Rust side needs to show and center the window (similar to `show_onboarding_window`). The view calls a Tauri command on mount to resize/center/show the window.

### 2. StartupReminder View

**New file:** `src/views/StartupReminder.vue`

**Layout:** A centered card (~380×280 logical pixels):
- App logo + app name at top
- One-line hint text: "Press {shortcut} to open the input" (i18n, parameterized)
- Close button (×) in top-right corner, displaying countdown number (10 → 0), decrementing each second

**Behavior:**
- `onMounted`:
  - Call `invoke('get_shortcut_label')` to fetch shortcut display text
  - Call `invoke('show_startup_reminder_window')` to center and show the window
  - Start 10s `setInterval` countdown
- Timer reaches 0 → `invoke('hide_main_window')`
- Click × → `invoke('hide_main_window')`
- Listen for `shortcut-triggered` event via `useShortcutTriggered` → `invoke('hide_main_window')`

**Shortcut interaction:** The Rust shortcut handler currently toggles visibility when the window is visible (hides it) and sets `window.location.hash = '/'` to navigate to floating-input. When on the reminder screen:
- Window is visible → shortcut press → window hides, route changes to `/` → desired behavior, no Rust changes needed.

### 3. Configuration Field

**`AppConfig` new field:**

| Layer | Field | Type | Default |
|-------|-------|------|---------|
| Rust `config.rs` | `show_startup_reminder` | `bool` | `true` (`#[serde(default = "default_true")]`) |
| TS `config.ts` | `show_startup_reminder` | `boolean` | `true` |

### 4. Settings Toggle

**Location:** Settings > General > Interface section, below Floating Window Opacity, as a new `card-row`.

- **Label (EN):** `Show shortcut hint on launch`
- **Label (ZH):** `启动时显示快捷键提示`
- **i18n key:** `settings.showShortcutHintLabel`
- **Control:** Toggle switch (reusing `ToggleLeft`/`ToggleRight` icons, already imported in Settings.vue)
- **Binding:** `appConfig.show_startup_reminder` (reactive, persisted via existing watch mechanism)

### 5. Rust `get_shortcut_label` Command

**New Tauri command** added to `src-tauri/src/commands/config_cmd.rs`:

```rust
#[tauri::command]
pub fn get_shortcut_label() -> String {
    "Alt+Y".to_string()
}
```

Single source of truth for the shortcut display text. When the shortcut becomes configurable in the future, only this function needs to change.

**Registration:** Add to `invoke_handler` in `lib.rs`.

### 6. Onboarding Shortcut Text Update

**Current:** `Onboarding.vue` hardcodes `shortcutKey` based on `navigator.userAgent` Mac detection:

```ts
const shortcutKey = computed(() => {
  const isMac = navigator.userAgent.includes("Mac");
  return isMac ? "⌥Y" : "Alt+Y";
});
```

**Change to:** `invoke('get_shortcut_label')` on mount, store in a ref. Both Onboarding and StartupReminder will use the same command.

### 7. i18n Keys

**New keys added to both `en.json` and `zh-CN.json`:**

```jsonc
// settings section
"showShortcutHintLabel": "Show shortcut hint on launch" / "启动时显示快捷键提示"

// new startupReminder section
"startupReminder": {
  "hint": "Press {shortcut} to open the input" / "按 {shortcut} 调出输入框"
}
```

## Files Changed

| File | Change |
|------|--------|
| `src/views/StartupReminder.vue` | **New** — reminder screen view |
| `src/router/index.ts` | Add `/startup-reminder` route, update guard logic |
| `src/stores/config.ts` | Add `show_startup_reminder` to `AppConfig` interface and defaults |
| `src/views/Settings.vue` | Add toggle row in Interface section |
| `src/views/Onboarding.vue` | Replace hardcoded `shortcutKey` with `invoke('get_shortcut_label')` |
| `src-tauri/src/config.rs` | Add `show_startup_reminder` field with `default_true` |
| `src-tauri/src/commands/config_cmd.rs` | Add `get_shortcut_label` command |
| `src-tauri/src/commands/window.rs` | Add `show_startup_reminder_window` command (380×280 logical, center + show) |
| `src-tauri/src/lib.rs` | Register new commands in `invoke_handler` |
| `src/locales/en.json` | Add i18n keys |
| `src/locales/zh-CN.json` | Add i18n keys |
