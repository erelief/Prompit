# Startup Shortcut Reminder — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a startup reminder screen that shows the global shortcut key when the app launches (non-first-run), with auto-close countdown and a Settings toggle.

**Architecture:** New Vue view (`StartupReminder.vue`) with its own route, following the same pattern as `Onboarding.vue`. Rust-side adds `get_shortcut_label` command for dynamic shortcut text and `show_startup_reminder_window` command for window positioning. A new `show_startup_reminder` config field (default `true`) controls the feature.

**Tech Stack:** Vue 3, Vue Router, Tauri v2 (Rust), vue-i18n, Tailwind CSS, Lucide icons

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src-tauri/src/config.rs` | Add `show_startup_reminder` field to Rust `AppConfig` |
| `src-tauri/src/commands/config_cmd.rs` | Add `get_shortcut_label` command |
| `src-tauri/src/commands/window.rs` | Add `show_startup_reminder_window` command |
| `src-tauri/src/lib.rs` | Register new commands in `invoke_handler` |
| `src/stores/config.ts` | Add `show_startup_reminder` to TS `AppConfig` and defaults |
| `src/views/StartupReminder.vue` | **New** — reminder screen view |
| `src/router/index.ts` | Add route and update guard logic |
| `src/main.ts` | Show reminder window on non-first-run startup |
| `src/views/Settings.vue` | Add toggle in Interface section |
| `src/views/Onboarding.vue` | Replace hardcoded `shortcutKey` with `invoke('get_shortcut_label')` |
| `src/locales/en.json` | Add i18n keys |
| `src/locales/zh-CN.json` | Add i18n keys |

---

### Task 1: Rust — Add config field and commands

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/commands/config_cmd.rs`
- Modify: `src-tauri/src/commands/window.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `show_startup_reminder` to Rust AppConfig**

In `src-tauri/src/config.rs`, add a `default_true` helper function and the new field to the `AppConfig` struct. Add it between `floating_opacity` and the existing `impl Default`:

```rust
// Add this helper function near the other default_* functions (around line 108):
fn default_true() -> bool {
    true
}
```

Then add the field to the `AppConfig` struct (after `floating_opacity`, around line 100):

```rust
    #[serde(default = "default_true")]
    pub show_startup_reminder: bool,
```

And add to the `Default` impl (after `floating_opacity: 85,`):

```rust
            show_startup_reminder: true,
```

Also update the test in `mod tests` — add `show_startup_reminder: true,` to both `AppConfig` structs in tests if they construct manually. The `test_config_roundtrip_via_json` test constructs manually, so add it there.

- [ ] **Step 2: Add `get_shortcut_label` command**

In `src-tauri/src/commands/config_cmd.rs`, add at the end (before `#[cfg(test)]`):

```rust
#[tauri::command]
pub fn get_shortcut_label() -> String {
    "Alt+Y".to_string()
}
```

- [ ] **Step 3: Add `show_startup_reminder_window` command**

In `src-tauri/src/commands/window.rs`, add after `show_onboarding_window`:

```rust
#[tauri::command]
pub fn show_startup_reminder_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let scale = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);

    let w = (380.0 * scale) as u32;
    let h = (280.0 * scale) as u32;

    window
        .set_size(tauri::PhysicalSize { width: w, height: h })
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: Register new commands in lib.rs**

In `src-tauri/src/lib.rs`, add to the `invoke_handler` macro (after `get_config_dir` and `set_onboarding_complete`):

```rust
            commands::config_cmd::get_shortcut_label,
```

And add after `show_onboarding_window`:

```rust
            commands::window::show_startup_reminder_window,
```

- [ ] **Step 5: Build Rust to verify compilation**

Run: `cd c:/Users/Hong/Documents/Works_temp/prompit && npx tauri build --debug 2>&1 | head -30` (or `cargo check` from `src-tauri/`)
Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/commands/config_cmd.rs src-tauri/src/commands/window.rs src-tauri/src/lib.rs
git commit -m "feat(startup-reminder): add Rust config field, get_shortcut_label and show_startup_reminder_window commands"
```

---

### Task 2: Frontend config + i18n keys

**Files:**
- Modify: `src/stores/config.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh-CN.json`

- [ ] **Step 1: Add `show_startup_reminder` to TS AppConfig**

In `src/stores/config.ts`, add to the `AppConfig` interface (after `floating_opacity`):

```typescript
  show_startup_reminder: boolean;
```

Add to `defaultConfig` (after `floating_opacity: 85,`):

```typescript
  show_startup_reminder: true,
```

- [ ] **Step 2: Add i18n keys to `en.json`**

In `src/locales/en.json`, add to the `"settings"` object (after `"resetToDefault"`):

```json
    "showShortcutHintLabel": "Show shortcut hint on launch",
```

Add a new top-level section (after `"onboarding"`):

```json
  "startupReminder": {
    "hint": "Press {shortcut} to open the input"
  }
```

- [ ] **Step 3: Add i18n keys to `zh-CN.json`**

In `src/locales/zh-CN.json`, add to the `"settings"` object (after `"resetToDefault"`):

```json
    "showShortcutHintLabel": "启动时显示快捷键提示",
```

Add a new top-level section (after `"onboarding"`):

```json
  "startupReminder": {
    "hint": "按 {shortcut} 调出输入框"
  }
```

- [ ] **Step 4: Commit**

```bash
git add src/stores/config.ts src/locales/en.json src/locales/zh-CN.json
git commit -m "feat(startup-reminder): add show_startup_reminder to TS config and i18n keys"
```

---

### Task 3: Router — Add route and update guard

**Files:**
- Modify: `src/router/index.ts`

- [ ] **Step 1: Add `/startup-reminder` route and update guard**

Replace the entire routes array and `beforeEach` guard in `src/router/index.ts` with:

```typescript
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "floating-input",
      component: () => import("../views/FloatingInput.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/Settings.vue"),
    },
    {
      path: "/settings/dictionary",
      name: "dictionary",
      component: () => import("../views/DictionaryEditor.vue"),
    },
    {
      path: "/settings/about",
      name: "about",
      component: () => import("../views/About.vue"),
    },
    {
      path: "/settings/reset",
      name: "reset",
      component: () => import("../views/ResetSoftware.vue"),
    },
    {
      path: "/onboarding",
      name: "onboarding",
      component: () => import("../views/Onboarding.vue"),
    },
    {
      path: "/startup-reminder",
      name: "startup-reminder",
      component: () => import("../views/StartupReminder.vue"),
    },
  ],
});

router.beforeEach((to) => {
  // First-run: always go to onboarding
  if (appConfig.providers.length === 0 && to.name !== "onboarding") {
    return { name: "onboarding" };
  }
  // Non-first-run: block onboarding
  if (appConfig.providers.length > 0 && to.name === "onboarding") {
    return { name: "floating-input" };
  }
});
```

Note: the decision to show `/startup-reminder` vs `/` is made in `main.ts` (next task), not in the router guard. The guard only handles the first-run vs non-first-run boundary.

- [ ] **Step 2: Update `main.ts` startup logic**

Replace the `router.isReady().then(...)` block in `src/main.ts` with:

```typescript
router.isReady().then(async () => {
  // Load config first so theme is known before first paint
  await loadConfig();
  initTheme();

  // Decide initial route based on state
  if (appConfig.providers.length === 0) {
    // First-run: show onboarding
    router.replace("/onboarding");
  } else if (appConfig.show_startup_reminder) {
    // Non-first-run with reminder enabled
    router.replace("/startup-reminder");
  }
  // else: stays on "/" (floating-input, window hidden)

  applyRouteTheme(router.currentRoute.value.path);
  app.mount("#app");

  // Show window immediately for onboarding
  if (appConfig.providers.length === 0) {
    invoke("show_onboarding_window");
  }
});
```

Note: `main.ts` has an `applyRouteTheme` function that checks specific paths. `/startup-reminder` needs settings-style background (not transparent). Update the `isSettings` check to include it:

```typescript
  const isSettings = path === "/settings" || path === "/settings/dictionary" || path === "/onboarding" || path === "/startup-reminder";
```

- [ ] **Step 3: Commit**

```bash
git add src/router/index.ts src/main.ts
git commit -m "feat(startup-reminder): add /startup-reminder route and update startup logic"
```

---

### Task 4: Create StartupReminder view

**Files:**
- Create: `src/views/StartupReminder.vue`

- [ ] **Step 1: Create `src/views/StartupReminder.vue`**

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { isDark } from "../composables/useTheme";
import { X } from "@lucide/vue";

const { t } = useI18n();

const shortcutLabel = ref("...");
const countdown = ref(10);
let timer: ReturnType<typeof setInterval> | null = null;

function close() {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
  invoke("hide_main_window");
}

useShortcutTriggered(() => {
  close();
});

onMounted(async () => {
  shortcutLabel.value = await invoke<string>("get_shortcut_label");
  invoke("show_startup_reminder_window");

  timer = setInterval(() => {
    countdown.value--;
    if (countdown.value <= 0) {
      close();
    }
  }, 1000);
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
  }
});
</script>

<template>
  <div class="reminder-root" :class="{ dark: isDark() }">
    <div class="reminder-card">
      <!-- Close button with countdown -->
      <button class="close-btn" @click="close" :title="t('common.hide')">
        <X :size="14" :stroke-width="2" />
        <span class="countdown-text">{{ countdown }}</span>
      </button>

      <!-- Content -->
      <img class="reminder-logo" src="/prompit_logo.svg" alt="" />
      <span class="reminder-name">Prompit</span>
      <p class="reminder-hint">{{ t('startupReminder.hint', { shortcut: shortcutLabel }) }}</p>
    </div>
  </div>
</template>

<style scoped>
.reminder-root {
  height: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}

.reminder-card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 32px 40px 28px;
}

.close-btn {
  position: absolute;
  top: -4px;
  right: -12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: 0.15s;
}

.close-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

.countdown-text {
  font-size: 10px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--color-text-muted);
}

.reminder-logo {
  height: 2em;
  width: auto;
  margin-bottom: 4px;
}

.reminder-name {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--color-text);
}

.reminder-hint {
  margin-top: 8px;
  font-size: 13px;
  color: var(--color-text-secondary);
  text-align: center;
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/views/StartupReminder.vue
git commit -m "feat(startup-reminder): create StartupReminder view with countdown and shortcut hint"
```

---

### Task 5: Settings — Add toggle in Interface section

**Files:**
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: Add toggle row in the Interface section**

Find the opacity `card-row` that ends around line 780 (after the `</div>` that closes the opacity `card-row`). Insert after that closing `</div>` and before the Language `card-row`:

```vue
          <!-- Show Shortcut Hint on Launch -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.showShortcutHintLabel') }}</span>
            <button class="toggle-btn" @click="appConfig.show_startup_reminder = !appConfig.show_startup_reminder">
              <ToggleRight v-if="appConfig.show_startup_reminder" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
          </div>
```

Note: `ToggleLeft` and `ToggleRight` are already imported in Settings.vue.

- [ ] **Step 2: Add toggle button CSS**

Find the existing toggle button styles. The `autoUpdate` toggle in the About section uses `about-auto-btn` class. Add a reusable `.toggle-btn` style. Insert near the `.card-row` styles (in the `<style scoped>` section):

```css
.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: none;
  cursor: pointer;
  padding: 2px;
  border-radius: 5px;
  transition: 0.15s;
}
.toggle-btn:hover {
  background: var(--color-surface-hover);
}
```

- [ ] **Step 3: Commit**

```bash
git add src/views/Settings.vue
git commit -m "feat(startup-reminder): add shortcut hint toggle in Settings > Interface"
```

---

### Task 6: Onboarding — Replace hardcoded shortcut text

**Files:**
- Modify: `src/views/Onboarding.vue`

- [ ] **Step 1: Replace hardcoded `shortcutKey` with dynamic fetch**

Find in `src/views/Onboarding.vue` (around line 119-122):

```typescript
const shortcutKey = computed(() => {
  const isMac = navigator.userAgent.includes("Mac");
  return isMac ? "⌥Y" : "Alt+Y";
});
```

Replace with:

```typescript
const shortcutKey = ref("...");
onMounted(async () => {
  try {
    shortcutKey.value = await invoke<string>("get_shortcut_label");
  } catch {
    shortcutKey.value = "Alt+Y";
  }
```

Note: The existing `onMounted` in Onboarding.vue starts around line 258. Merge the `invoke` call into the existing `onMounted` instead of creating a second one. Add `invoke("get_shortcut_label").then(s => { shortcutKey.value = s; }).catch(() => {});` as the first line inside the existing `onMounted` callback.

The full replacement for line 119-122 becomes:

```typescript
const shortcutKey = ref("...");
```

And inside the existing `onMounted` (around line 258), add as the first line:

```typescript
  invoke<string>("get_shortcut_label").then(s => { shortcutKey.value = s; }).catch(() => {});
```

Make sure `invoke` is already imported in Onboarding.vue — it is (used for `show_onboarding_window`, `set_onboarding_complete`, etc.).

- [ ] **Step 2: Commit**

```bash
git add src/views/Onboarding.vue
git commit -m "refactor(onboarding): replace hardcoded shortcut text with get_shortcut_label command"
```

---

### Task 7: Smoke test

**Files:** None (testing only)

- [ ] **Step 1: Run dev server**

Run: `cd c:/Users/Hong/Documents/Works_temp/prompit && npx tauri dev`

- [ ] **Step 2: Test first-run behavior**

Delete or rename the config file to simulate first-run. Launch the app. Expected: Onboarding wizard appears (no change from current behavior). Complete onboarding.

- [ ] **Step 3: Test startup reminder**

Close the app completely (via tray exit). Relaunch. Expected: Startup reminder screen appears at center with shortcut text and 10s countdown.

- [ ] **Step 4: Test close button**

Click the × button. Expected: Window hides immediately.

- [ ] **Step 5: Test auto-close**

Relaunch. Wait 10 seconds without interaction. Expected: Window auto-hides after countdown reaches 0.

- [ ] **Step 6: Test shortcut close**

Relaunch. Press Alt+Y. Expected: Window hides (shortcut closes it).

- [ ] **Step 7: Test toggle**

Open Settings (via tray). Go to General > Interface. Toggle off "Show shortcut hint on launch". Exit app. Relaunch. Expected: No reminder screen, window stays hidden.

- [ ] **Step 8: Toggle back on and verify**

Re-enable the toggle. Exit and relaunch. Expected: Reminder screen appears again.
