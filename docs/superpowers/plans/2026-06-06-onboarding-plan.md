# Onboarding Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a 5-step onboarding wizard that guides first-time users through adding an LLM provider and selecting a model, triggered whenever no providers are configured.

**Architecture:** Single Vue component (`Onboarding.vue`) on a `/onboarding` route with internal step management. Router guard redirects to onboarding when `providers` is empty. Rust-side `onboarding_complete` flag disables the global shortcut during onboarding.

**Tech Stack:** Vue 3 + TypeScript + vue-i18n, Tauri 2 (Rust), Tailwind CSS v4

**Spec:** `docs/superpowers/specs/2026-06-06-onboarding-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/state.rs` | Modify | Add `onboarding_complete` AtomicBool to app state |
| `src-tauri/src/commands/config_cmd.rs` | Modify | Add `set_onboarding_complete` command |
| `src-tauri/src/lib.rs` | Modify | Register new command, manage new state |
| `src-tauri/src/shortcut.rs` | Modify | Check `onboarding_complete` before handling shortcut |
| `src/services/llm-client.ts` | Modify | Extract `testProviderConnection` and `fetchProviderModels` |
| `src/views/Settings.vue` | Modify | Use extracted shared functions |
| `src/locales/en.json` | Modify | Add `onboarding.*` keys |
| `src/locales/zh-CN.json` | Modify | Add `onboarding.*` keys |
| `src/router/index.ts` | Modify | Add `/onboarding` route + navigation guard |
| `src/views/Onboarding.vue` | Create | Main onboarding wizard component |
| `src/main.ts` | Modify | Add `/onboarding` to `applyRouteTheme` |

---

### Task 1: Create feature branch

- [ ] **Step 1: Branch from master**

```bash
git checkout -b feat/onboarding
```

---

### Task 2: Add `onboarding_complete` state and command (Rust)

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/commands/config_cmd.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `OnboardingState` to `state.rs`**

Add after the `WindowConfig` struct (after line 36):

```rust
pub struct OnboardingState {
    pub complete: AtomicBool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            complete: AtomicBool::new(false),
        }
    }
}

impl OnboardingState {
    pub fn set_complete(&self, value: bool) {
        self.complete.store(value, Ordering::Relaxed);
    }

    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Relaxed)
    }
}
```

- [ ] **Step 2: Add command to `config_cmd.rs`**

Add at the end of the file:

```rust
#[tauri::command]
pub fn set_onboarding_complete(app: AppHandle) {
    if let Some(state) = app.try_state::<crate::state::OnboardingState>() {
        state.set_complete(true);
    }
}
```

- [ ] **Step 3: Register state and command in `lib.rs`**

In the `run()` function, add `.manage(state::OnboardingState::default())` after the existing `.manage(state::WindowConfig::default())` (line 94).

Add `commands::config_cmd::set_onboarding_complete` to the `invoke_handler` `generate_handler!` list.

- [ ] **Step 4: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/commands/config_cmd.rs src-tauri/src/lib.rs
git commit -m "feat(rust): add onboarding_complete state and command"
```

---

### Task 3: Guard shortcut with `onboarding_complete` flag

**Files:**
- Modify: `src-tauri/src/shortcut.rs`

- [ ] **Step 1: Add guard to shortcut handler**

In `shortcut.rs`, inside the `on_shortcut` closure, add a check at the very beginning (after `if event.state() == ShortcutState::Pressed {`, before the `let main_window` line):

```rust
// Skip shortcut during onboarding
if let Some(state) = app_handle.try_state::<crate::state::OnboardingState>() {
    if !state.is_complete() {
        return;
    }
}
```

- [ ] **Step 2: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/shortcut.rs
git commit -m "feat(rust): disable global shortcut during onboarding"
```

---

### Task 4: Extract shared connection/model functions from Settings.vue

**Files:**
- Modify: `src/services/llm-client.ts`
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: Add shared functions to `llm-client.ts`**

Add at the end of `src/services/llm-client.ts`, before the closing:

```typescript
export async function testProviderConnection(
  provider: Pick<ProviderConfig, "api_key" | "base_url" | "api_format">
): Promise<{ ok: boolean; status?: number; error?: string }> {
  if (!provider.api_key || !provider.base_url) {
    return { ok: false, error: "Missing API key or base URL" };
  }
  try {
    const fmt = resolveFormat(provider.api_format);
    const url = provider.base_url.replace(/\/$/, "");
    const headers: Record<string, string> = {};
    if (fmt.auth_header && provider.api_key) {
      headers[fmt.auth_header] = `${fmt.auth_prefix}${provider.api_key}`;
    }
    for (const [k, v] of Object.entries(fmt.extra_headers)) {
      headers[k] = v;
    }
    const modelsEndpoint = fmt.models_endpoint || "/models";
    const r = await fetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (r.ok) {
      return { ok: true };
    } else {
      await r.text();
      return { ok: false, status: r.status, error: `Failed (${r.status})` };
    }
  } catch {
    return { ok: false, error: "Connection failed" };
  }
}

export async function fetchProviderModels(
  provider: Pick<ProviderConfig, "api_key" | "base_url" | "api_format">
): Promise<{ ok: boolean; models?: string[]; error?: string }> {
  if (!provider.api_key || !provider.base_url) {
    return { ok: false, error: "Missing API key or base URL" };
  }
  try {
    const fmt = resolveFormat(provider.api_format);
    const url = provider.base_url.replace(/\/$/, "");
    const headers: Record<string, string> = {};
    if (fmt.auth_header && provider.api_key) {
      headers[fmt.auth_header] = `${fmt.auth_prefix}${provider.api_key}`;
    }
    for (const [k, v] of Object.entries(fmt.extra_headers)) {
      headers[k] = v;
    }
    const modelsEndpoint = fmt.models_endpoint || "/models";
    const r = await fetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const data = await r.json();

    const modelsListPath = fmt.response["models_list"];
    let modelIds: string[];
    if (modelsListPath) {
      const raw = resolvePath(data, modelsListPath.replace(/\.\*$/, ""));
      modelIds = Array.isArray(raw) ? raw.filter((m: any) => typeof m === "string").sort() : [];
    } else {
      modelIds = data.data?.map((m: any) => m.id).sort() || [];
    }
    return { ok: true, models: modelIds };
  } catch (err) {
    return {
      ok: false,
      error: `Fetch failed: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
}
```

Add the `ProviderConfig` import to the existing imports at the top of the file. The file currently imports from `../stores/config` — add `ProviderConfig` to that import:

```typescript
import { getActiveModel, appConfig, personaStore, loadDictionary } from "../stores/config";
import type { ApiFormat, ProviderConfig } from "../stores/config";
```

- [ ] **Step 2: Refactor Settings.vue to use shared functions**

In `src/views/Settings.vue`, add imports for the new shared functions:

```typescript
import { resolveFormat, resolvePath, testProviderConnection, fetchProviderModels } from "../services/llm-client";
```

Replace the `testConnection` function (lines 414-446) with:

```typescript
async function testConnection(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  testingProvider.value = index;
  const result = await testProviderConnection(provider);
  if (result.ok) {
    fetchStatuses.value.set(index, "Connected");
    setTimeout(() => fetchStatuses.value.delete(index), 3000);
  } else {
    fetchStatuses.value.set(index, result.error || "Connection failed");
    setTimeout(() => fetchStatuses.value.delete(index), 4000);
  }
  testingProvider.value = null;
}
```

Replace the `fetchModels` function (lines 448-487) with:

```typescript
async function fetchModels(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  fetchingProviders.value.add(index);
  const result = await fetchProviderModels(provider);
  if (result.ok && result.models) {
    fetchedModels.value.set(`p${index}`, result.models);
    fetchSuccess.value.add(index);
    setTimeout(() => { fetchSuccess.value.delete(index); fetchSuccess.value = new Set(fetchSuccess.value); }, 2000);
  } else {
    fetchStatuses.value.set(index, result.error || "Fetch failed");
    setTimeout(() => fetchStatuses.value.delete(index), 5000);
  }
  fetchingProviders.value.delete(index);
}
```

- [ ] **Step 3: Verify frontend builds**

Run: `npm run build`
Expected: no TypeScript errors

- [ ] **Step 4: Commit**

```bash
git add src/services/llm-client.ts src/views/Settings.vue
git commit -m "refactor: extract connection/model functions from Settings to llm-client"
```

---

### Task 5: Add i18n translation keys

**Files:**
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh-CN.json`

- [ ] **Step 1: Add English translations**

Add an `"onboarding"` section to `src/locales/en.json` (at the top level, after `"common"`):

```json
"onboarding": {
  "hello": "Hello",
  "selectLanguage": "Select your language",
  "welcomeTitle": "Welcome to Prompit",
  "infoTitle": "One more step",
  "infoBody": "Prompit uses AI models to translate text. You need to connect at least one model provider before you can start. Let's set one up now.",
  "addProviderTitle": "Add a provider",
  "preset": "Preset",
  "selectPreset": "Select a preset…",
  "providerName": "Provider name",
  "apiKey": "API Key",
  "baseUrl": "Base URL",
  "testAndContinue": "Test & Continue",
  "testingConnection": "Testing connection…",
  "fetchingModels": "Fetching models…",
  "connectionFailed": "Connection failed",
  "selectModelsTitle": "Select models",
  "selectModelsBody": "Choose at least one model to use for translation.",
  "selectAll": "Select all",
  "deselectAll": "Deselect all",
  "noModelsFound": "No models found.",
  "retryFetch": "Retry",
  "fetchFailed": "Failed to fetch models",
  "doneTitle": "You're all set!",
  "doneBody": "You can now start translating.",
  "shortcutHint": "Press {shortcut} to open the input window.",
  "addMoreProviders": "You can add more providers anytime in Settings.",
  "finish": "Finish",
  "next": "Next",
  "previous": "Previous",
  "back": "Back"
}
```

- [ ] **Step 2: Add Chinese translations**

Add the corresponding `"onboarding"` section to `src/locales/zh-CN.json`:

```json
"onboarding": {
  "hello": "Hello",
  "selectLanguage": "选择你的语言",
  "welcomeTitle": "欢迎使用 Prompit",
  "infoTitle": "还需一步",
  "infoBody": "Prompit 需要接入 AI 模型才能翻译文本。接下来会引导你添加一个模型供应商。",
  "addProviderTitle": "添加供应商",
  "preset": "预设",
  "selectPreset": "选择预设…",
  "providerName": "供应商名称",
  "apiKey": "API Key",
  "baseUrl": "Base URL",
  "testAndContinue": "测试并继续",
  "testingConnection": "正在测试连接…",
  "fetchingModels": "正在获取模型列表…",
  "connectionFailed": "连接失败",
  "selectModelsTitle": "选择模型",
  "selectModelsBody": "至少选择一个模型用于翻译。",
  "selectAll": "全选",
  "deselectAll": "取消全选",
  "noModelsFound": "未找到模型。",
  "retryFetch": "重试",
  "fetchFailed": "获取模型失败",
  "doneTitle": "设置完成！",
  "doneBody": "现在可以开始翻译了。",
  "shortcutHint": "按 {shortcut} 打开输入窗口。",
  "addMoreProviders": "你可以在设置中随时添加更多供应商。",
  "finish": "完成",
  "next": "下一步",
  "previous": "上一步",
  "back": "返回"
}
```

- [ ] **Step 3: Verify i18n validation passes**

Run: `node scripts/validate-i18n.mjs`
Expected: passes (both files have matching keys)

- [ ] **Step 4: Commit**

```bash
git add src/locales/en.json src/locales/zh-CN.json
git commit -m "feat(i18n): add onboarding translation keys"
```

---

### Task 6: Add onboarding route and navigation guard

**Files:**
- Modify: `src/router/index.ts`

- [ ] **Step 1: Update router with onboarding route and guard**

Replace the entire content of `src/router/index.ts` with:

```typescript
import { createRouter, createWebHashHistory } from "vue-router";
import { appConfig } from "../stores/config";

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
      path: "/onboarding",
      name: "onboarding",
      component: () => import("../views/Onboarding.vue"),
    },
  ],
});

router.beforeEach((to) => {
  if (appConfig.providers.length === 0 && to.name !== "onboarding") {
    return { name: "onboarding" };
  }
  if (appConfig.providers.length > 0 && to.name === "onboarding") {
    return { name: "floating-input" };
  }
});

export default router;
```

- [ ] **Step 2: Update `applyRouteTheme` in `main.ts`**

In `src/main.ts`, update the `applyRouteTheme` function to include `/onboarding`:

```typescript
function applyRouteTheme(path: string) {
  const isSettings = path === "/settings" || path === "/settings/dictionary" || path === "/onboarding";
  const bg = isSettings ? "var(--color-bg)" : "transparent";
  document.documentElement.style.background = bg;
  document.body.style.background = bg;
  document.body.style.overflow = isSettings ? "auto" : "hidden";
  document.getElementById("app")!.style.background = bg;
}
```

- [ ] **Step 3: Verify frontend builds**

Run: `npm run build`
Expected: no TypeScript errors (Onboarding.vue doesn't exist yet, but the lazy import will just fail at runtime, not at build time — verify this)

- [ ] **Step 4: Commit**

```bash
git add src/router/index.ts src/main.ts
git commit -m "feat: add onboarding route with navigation guard"
```

---

### Task 7: Create Onboarding.vue

**Files:**
- Create: `src/views/Onboarding.vue`

This is the main component. Before writing, invoke the `/frontend-design` skill to design the UI with the existing app style (Geist font, Tailwind CSS, Lucide icons, glass-morphism on floating input, settings page uses card-based layout).

- [ ] **Step 1: Invoke frontend-design skill for UI design**

Invoke the frontend-design skill with this context:
- App: Prompit, a Tauri desktop real-time translator
- Design system: Geist font, Tailwind CSS v4, Lucide Vue icons, off-white background (`var(--color-bg)`), card-based layout
- Current pages: FloatingInput (glass-morphism floating window), Settings (full-page with sidebar tabs)
- Task: Design a 5-step onboarding wizard with centered card layout, step indicator dots, directional slide transitions, modern and minimal aesthetic
- Steps: Welcome (Hello + lang select), Info (explainer), Add Provider (preset + form), Select Models (checkbox list), Done (congratulations)

- [ ] **Step 2: Write the `Onboarding.vue` component**

Create `src/views/Onboarding.vue` with the following structure. Use the design from Step 1 for the template and styling.

**Script section — key logic:**

```typescript
<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  appConfig,
  loadProviderPresets,
} from "../stores/config";
import type { ProviderConfig, ProviderPreset } from "../stores/config";
import {
  testProviderConnection,
  fetchProviderModels,
} from "../services/llm-client";
import { BUILTIN_LANGUAGES, getLangName } from "../constants/languages";
import {
  ChevronRight,
  ChevronLeft,
  Loader2,
  Check,
  Eye,
  EyeOff,
  Globe,
  Zap,
  PartyPopper,
} from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();

// ── Step management ──
const currentStep = ref(0);
const direction = ref<"forward" | "backward">("forward");

// ── Step 0: Language ──
const selectedLang = ref(appConfig.app_lang || "en");

// ── Step 2: Provider form ──
const providerForm = ref<ProviderConfig>({
  name: "",
  api_key: "",
  base_url: "",
  models: [],
  temperature: 0.3,
  max_tokens: 1024,
});
const providerPresets = ref<ProviderPreset[]>([]);
const selectedPreset = ref("");
const showApiKey = ref(false);

// ── Step 3: Models ──
const availableModels = ref<string[]>([]);
const selectedModels = ref(new Set<string>());
const fetchError = ref("");
const isConnecting = ref(false);
const isFetching = ref(false);

// ── Computed ──
const canProceed = computed(() => {
  switch (currentStep.value) {
    case 0: return true;
    case 1: return true;
    case 2:
      return (
        providerForm.value.name.trim() !== "" &&
        providerForm.value.api_key.trim() !== "" &&
        providerForm.value.base_url.trim() !== ""
      );
    case 3:
      return !isConnecting.value && !isFetching.value;
    case 4:
      return selectedModels.value.size > 0;
    default: return false;
  }
});

const isLastStep = computed(() => currentStep.value === 4);

// Platform-aware shortcut
const shortcutKey = computed(() => {
  const isMac = navigator.userAgent.includes("Mac");
  return isMac ? "⌥Y" : "Alt+Y";
});

// ── Navigation ──
function goNext() {
  if (!canProceed.value) return;
  if (currentStep.value === 2) {
    confirmProviderAndAdvance();
    return;
  }
  if (currentStep.value === 4) {
    finishOnboarding();
    return;
  }
  if (currentStep.value === 0) {
    applyLanguage();
  }
  direction.value = "forward";
  currentStep.value++;
}

function goPrev() {
  if (currentStep.value === 0) return;
  direction.value = "backward";
  currentStep.value--;
}

// ── Step 0 logic ──
function applyLanguage() {
  appConfig.app_lang = selectedLang.value;
}

// ── Step 2 logic ──
function applyPreset(presetName: string) {
  const preset = providerPresets.value.find((p) => p.name === presetName);
  if (!preset) return;
  selectedPreset.value = presetName;
  providerForm.value.name = preset.provider_name;
  providerForm.value.base_url = preset.base_url;
  providerForm.value.preset = presetName;
  providerForm.value.api_format = { ...preset.api_format };
}

// ── Step 3 flow ──
async function confirmProviderAndAdvance() {
  isConnecting.value = true;
  fetchError.value = "";

  const result = await testProviderConnection(providerForm.value);
  if (!result.ok) {
    fetchError.value = result.error || "Connection failed";
    isConnecting.value = false;
    return;
  }

  isConnecting.value = false;
  isFetching.value = true;

  const modelsResult = await fetchProviderModels(providerForm.value);
  if (!modelsResult.ok || !modelsResult.models || modelsResult.models.length === 0) {
    fetchError.value = modelsResult.error || "No models found";
    isFetching.value = false;
    return;
  }

  availableModels.value = modelsResult.models;
  isFetching.value = false;

  direction.value = "forward";
  currentStep.value = 3;
}

// ── Step 4 logic ──
function toggleModel(id: string) {
  const s = new Set(selectedModels.value);
  s.has(id) ? s.delete(id) : s.add(id);
  selectedModels.value = s;
}

function selectAll() {
  selectedModels.value = new Set(availableModels.value);
}

function deselectAll() {
  selectedModels.value = new Set();
}

async function finishOnboarding() {
  // Write provider to config
  providerForm.value.models = [...selectedModels.value].map((id) => ({ id }));
  appConfig.providers.push({ ...providerForm.value });
  appConfig.active_provider_index = 0;
  appConfig.active_model_index = 0;

  // Mark onboarding complete in Rust
  await invoke("set_onboarding_complete");

  // Navigate to main view
  router.replace("/");
}

// ── Init ──
onMounted(async () => {
  try {
    providerPresets.value = await loadProviderPresets();
  } catch (err) {
    console.error("Failed to load presets:", err);
  }
});
</script>
```

**Template section — use the frontend-design skill output for layout and styling.** Key structure:

```html
<template>
  <div class="onboarding">
    <div class="step-container">
      <Transition :name="direction === 'forward' ? 'slide-left' : 'slide-right'" mode="out-in">
        <!-- Step 0: Welcome -->
        <div v-if="currentStep === 0" key="step0" class="step">
          <h1 class="hello-text">{{ t('onboarding.hello') }}</h1>
          <p class="step-title">{{ t('onboarding.welcomeTitle') }}</p>
          <div class="lang-select">
            <label>{{ t('onboarding.selectLanguage') }}</label>
            <select v-model="selectedLang" class="select-input">
              <option v-for="lang in BUILTIN_LANGUAGES" :key="lang" :value="lang">
                {{ getLangName(lang) }}
              </option>
            </select>
          </div>
        </div>

        <!-- Step 1: Info -->
        <div v-else-if="currentStep === 1" key="step1" class="step">
          <Zap :size="48" class="step-icon" />
          <h2>{{ t('onboarding.infoTitle') }}</h2>
          <p class="info-text">{{ t('onboarding.infoBody') }}</p>
        </div>

        <!-- Step 2: Add Provider -->
        <div v-else-if="currentStep === 2" key="step2" class="step">
          <h2>{{ t('onboarding.addProviderTitle') }}</h2>
          <div class="form-group">
            <label>{{ t('onboarding.preset') }}</label>
            <select v-model="selectedPreset" @change="applyPreset(selectedPreset)" class="select-input">
              <option value="">{{ t('onboarding.selectPreset') }}</option>
              <option v-for="p in providerPresets" :key="p.name" :value="p.name">{{ p.name }}</option>
            </select>
          </div>
          <div class="form-group">
            <label>{{ t('onboarding.providerName') }}</label>
            <input v-model="providerForm.name" class="text-input" />
          </div>
          <div class="form-group">
            <label>{{ t('onboarding.apiKey') }}</label>
            <div class="input-with-toggle">
              <input v-model="providerForm.api_key" :type="showApiKey ? 'text' : 'password'" class="text-input" />
              <button class="icon-btn" @click="showApiKey = !showApiKey">
                <Eye v-if="!showApiKey" :size="16" />
                <EyeOff v-else :size="16" />
              </button>
            </div>
          </div>
          <div class="form-group">
            <label>{{ t('onboarding.baseUrl') }}</label>
            <input v-model="providerForm.base_url" class="text-input" />
          </div>
          <p v-if="fetchError" class="error-text">{{ fetchError }}</p>
          <p v-if="isConnecting" class="status-text">
            <Loader2 :size="14" class="spin" /> {{ t('onboarding.testingConnection') }}
          </p>
          <p v-if="isFetching" class="status-text">
            <Loader2 :size="14" class="spin" /> {{ t('onboarding.fetchingModels') }}
          </p>
        </div>

        <!-- Step 3: Select Models -->
        <div v-else-if="currentStep === 3" key="step3" class="step">
          <h2>{{ t('onboarding.selectModelsTitle') }}</h2>
          <p>{{ t('onboarding.selectModelsBody') }}</p>
          <div class="model-actions">
            <button class="link-btn" @click="selectAll">{{ t('onboarding.selectAll') }}</button>
            <span>·</span>
            <button class="link-btn" @click="deselectAll">{{ t('onboarding.deselectAll') }}</button>
          </div>
          <div class="model-list">
            <label v-for="m in availableModels" :key="m" class="model-item">
              <input type="checkbox" :checked="selectedModels.has(m)" @change="toggleModel(m)" />
              <span>{{ m }}</span>
            </label>
          </div>
          <p v-if="fetchError" class="error-text">{{ fetchError }}</p>
        </div>

        <!-- Step 4: Done -->
        <div v-else-if="currentStep === 4" key="step4" class="step">
          <PartyPopper :size="48" class="step-icon" />
          <h2>{{ t('onboarding.doneTitle') }}</h2>
          <p>{{ t('onboarding.doneBody') }}</p>
          <p class="shortcut-hint">{{ t('onboarding.shortcutHint', { shortcut: shortcutKey }) }}</p>
          <p class="tip-text">{{ t('onboarding.addMoreProviders') }}</p>
        </div>
      </Transition>
    </div>

    <!-- Bottom bar -->
    <div class="nav-bar">
      <div class="step-dots">
        <span v-for="i in 5" :key="i" class="dot" :class="{ active: currentStep === i - 1, done: currentStep > i - 1 }" />
      </div>
      <div class="nav-buttons">
        <button v-if="currentStep > 0" class="btn btn-secondary" @click="goPrev">
          <ChevronLeft :size="16" /> {{ t('onboarding.previous') }}
        </button>
        <button class="btn btn-primary" :disabled="!canProceed || isConnecting || isFetching" @click="goNext">
          <Loader2 v-if="isConnecting || isFetching" :size="16" class="spin" />
          <template v-else>
            {{ isLastStep ? t('onboarding.finish') : currentStep === 2 ? t('onboarding.testAndContinue') : t('onboarding.next') }}
            <ChevronRight v-if="!isLastStep" :size="16" />
            <Check v-else :size="16" />
          </template>
        </button>
      </div>
    </div>
  </div>
</template>
```

**Style section:** Use Tailwind CSS classes where possible, scoped `<style scoped>` for transitions and custom styles. The frontend-design skill should be invoked here for the detailed visual design.

- [ ] **Step 3: Verify frontend builds**

Run: `npm run build`
Expected: no TypeScript errors

- [ ] **Step 4: Commit**

```bash
git add src/views/Onboarding.vue
git commit -m "feat: add onboarding wizard component"
```

---

### Task 8: End-to-end verification

- [ ] **Step 1: Build the full Tauri app**

Run: `npm run tauri build` (or `npm run tauri dev` for development)
Expected: app compiles and launches

- [ ] **Step 2: Test onboarding flow**

1. Delete config.json (or use sandbox mode: `npm run tauri:sandbox`)
2. Launch app → should redirect to onboarding
3. Step 0: Select a language → text updates
4. Step 1: Read info → Next works
5. Step 2: Select preset → fields auto-fill → enter API key → Next triggers test
6. Step 3: Models list appears → select models → Finish
7. Step 4: Shows done screen → Finish navigates to `/`
8. Verify: `Alt+Y` now works (toggles floating input)
9. Verify: Settings page shows the added provider

- [ ] **Step 3: Test shortcut guard**

1. While on onboarding (step 0-3), press `Alt+Y`
2. Expected: nothing happens (window does not appear)

- [ ] **Step 4: Test re-trigger**

1. Go to Settings → delete all providers
2. Expected: redirects back to onboarding

- [ ] **Step 5: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: address onboarding flow issues from manual testing"
```
