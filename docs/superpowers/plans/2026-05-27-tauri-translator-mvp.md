# Tauri 跨平台打字翻译器 MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the existing AHK typing translator as a cross-platform Tauri 2.0 desktop app with global shortcut, floating input window, LLM-powered translation, and paste injection.

**Architecture:** Tauri 2.0 app with Vue 3 + TypeScript + TailwindCSS frontend. Rust backend handles OS-level operations (global shortcut, clipboard/paste simulation, window management). Frontend owns business logic (LLM API calls, config UI, translation flow). Two windows: a borderless transparent floating input (always-on-top, skip taskbar) and a standard settings window.

**Tech Stack:** Tauri 2.x, Vue 3, TypeScript, Vite, TailwindCSS v4, Vue Router, reqwest, serde, enigo, tauri-plugin-global-shortcut, tauri-plugin-clipboard-manager

---

## File Structure

```
realtime-translator-tauri/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json            # Window definitions, app metadata
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json           # Permissions for plugins + IPC
│   ├── icons/
│   │   └── icon.png               # App icon
│   └── src/
│       ├── main.rs                # Entry point: plugin registration, app setup
│       ├── lib.rs                 # Module re-exports
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── window.rs          # hide_main_window, show_main_window, open_settings_window
│       │   ├── clipboard.rs       # simulate_paste (clipboard backup → write → Ctrl+V → restore)
│       │   └── config_cmd.rs      # read_config, save_config Tauri commands
│       ├── shortcut.rs            # Global shortcut registration + event emission
│       ├── config.rs              # Config struct, serde, file I/O logic
│       └── aifw.rs                # AIFW subprocess lifecycle (Phase 4)
├── src/
│   ├── main.ts                    # Vue app creation, router mount
│   ├── App.vue                    # <RouterView> wrapper
│   ├── router/
│   │   └── index.ts               # / → floating-input, /settings → settings
│   ├── views/
│   │   ├── FloatingInput.vue      # Textarea + status display + settings button
│   │   └── Settings.vue           # Config form with save
│   ├── stores/
│   │   └── config.ts              # Reactive config store with load/save
│   ├── services/
│   │   └── llm-client.ts          # LLM HTTP client (OpenAI-compatible + AIFW routing)
│   ├── composables/
│   │   └── useTauriEvents.ts      # Shortcut event listener composable
│   ├── style.css                  # TailwindCSS import + global styles
│   └── vite-env.d.ts
├── index.html                     # HTML entry
├── package.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── tailwind.config.ts             # (v3 style) or CSS-only (v4)
└── postcss.config.js
```

**Responsibilities:**

| File | Responsibility |
|---|---|
| `src-tauri/src/main.rs` | Plugin registration, app builder |
| `src-tauri/src/commands/window.rs` | Window visibility toggling |
| `src-tauri/src/commands/clipboard.rs` | Clipboard backup/write/paste/restore |
| `src-tauri/src/commands/config_cmd.rs` | Read/write config.json via Tauri fs |
| `src-tauri/src/shortcut.rs` | Register Alt+Y, emit `shortcut_triggered` event |
| `src-tauri/src/config.rs` | Config schema + serialization |
| `src-tauri/src/aifw.rs` | AIFW subprocess spawn/kill (Phase 4) |
| `src/views/FloatingInput.vue` | Main floating input UI |
| `src/views/Settings.vue` | Settings form UI |
| `src/stores/config.ts` | Reactive config state + Tauri IPC load/save |
| `src/services/llm-client.ts` | HTTP client for translation API |
| `src/composables/useTauriEvents.ts` | Listen to `shortcut_triggered` |

---

## Phase 1: Project Initialization & Window Setup

### Task 1: Initialize Tauri 2 Project with Vue 3

**Files:**
- Create: `realtime-translator-tauri/` (entire project scaffold)
- Create: `realtime-translator-tauri/package.json`
- Create: `realtime-translator-tauri/src-tauri/Cargo.toml`
- Create: `realtime-translator-tauri/src-tauri/tauri.conf.json`
- Create: `realtime-translator-tauri/src/main.ts`
- Create: `realtime-translator-tauri/src/App.vue`

- [ ] **Step 1: Install Tauri CLI (if not already installed)**

```powershell
cargo install create-tauri-app
```

- [ ] **Step 2: Scaffold the project**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit
cargo create-tauri-app realtime-translator-tauri --template vue-ts --manager npm
```

Expected: A new directory `realtime-translator-tauri/` with Tauri 2 + Vue 3 + TypeScript + Vite scaffold.

- [ ] **Step 3: Install dependencies and verify dev build works**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm install
npm run tauri dev
```

Expected: A default Tauri window opens showing the Vue template. Press Ctrl+C to stop.

- [ ] **Step 4: Add TailwindCSS v4**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm install tailwindcss @tailwindcss/vite
```

- [ ] **Step 5: Configure TailwindCSS in Vite**

Edit `vite.config.ts` to add the Tailwind plugin:

```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  clearScreen: false,
  server: {
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
});
```

Replace `src/style.css` content:

```css
@import "tailwindcss";
```

Replace `src/App.vue` content:

```vue
<script setup lang="ts">
</script>

<template>
  <router-view />
</template>
```

- [ ] **Step 6: Add Vue Router**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm install vue-router
```

Create `src/router/index.ts`:

```typescript
import { createRouter, createWebHashHistory } from "vue-router";

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
  ],
});

export default router;
```

Update `src/main.ts`:

```typescript
import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./style.css";

createApp(App).use(router).mount("#app");
```

- [ ] **Step 7: Create placeholder view files**

Create `src/views/FloatingInput.vue`:

```vue
<script setup lang="ts">
</script>

<template>
  <div class="p-4">
    <h1>Floating Input</h1>
  </div>
</template>
```

Create `src/views/Settings.vue`:

```vue
<script setup lang="ts">
</script>

<template>
  <div class="p-4">
    <h1>Settings</h1>
  </div>
</template>
```

- [ ] **Step 8: Verify dev build works with router**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected: Window shows "Floating Input" text (from the `/` route). Navigate to `#/settings` shows "Settings".

- [ ] **Step 9: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git init
git add -A
git commit -m "chore: initialize Tauri 2 + Vue 3 + TypeScript project with TailwindCSS and Vue Router"
```

---

### Task 2: Configure Main Window as Transparent Overlay

**Files:**
- Modify: `realtime-translator-tauri/src-tauri/tauri.conf.json`
- Modify: `realtime-translator-tauri/src-tauri/Cargo.toml`
- Modify: `realtime-translator-tauri/src-tauri/src/main.rs`
- Modify: `realtime-translator-tauri/src/style.css`
- Modify: `realtime-translator-tauri/index.html`

- [ ] **Step 1: Configure main window properties in tauri.conf.json**

Edit `src-tauri/tauri.conf.json` — replace the `windows` array:

```json
{
  "productName": "realtime-translator",
  "version": "0.1.0",
  "identifier": "com.translator.realtime",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Translator",
        "width": 600,
        "height": 200,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "resizable": false,
        "visible": false,
        "center": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "icon": ["icons/icon.png"]
  }
}
```

- [ ] **Step 2: Update capabilities for window and plugin permissions**

Create or edit `src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "description": "Default capabilities for the translator app",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-center",
    "core:window:allow-set-always-on-top",
    "core:window:allow-create",
    "core:window:allow-get-all-windows",
    "shell:default"
  ]
}
```

- [ ] **Step 3: Make body and html transparent in index.html**

Edit `index.html` — add inline styles:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Translator</title>
    <style>
      html, body {
        background: transparent;
        margin: 0;
        padding: 0;
        overflow: hidden;
      }
    </style>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 4: Add transparent background to style.css**

Update `src/style.css`:

```css
@import "tailwindcss";

#app {
  background: transparent;
}
```

- [ ] **Step 5: Write a Rust test for window config deserialization**

Create `src-tauri/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub display_name: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub models: Vec<ModelConfig>,
    #[serde(default)]
    pub selected_model_index: usize,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
    #[serde(default)]
    pub privacy_mode: bool,
    #[serde(default = "default_translation_mode")]
    pub translation_mode: String,
    #[serde(default)]
    pub persona: String,
}

fn default_target_lang() -> String {
    "English".to_string()
}

fn default_translation_mode() -> String {
    "manual".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            models: vec![],
            selected_model_index: 0,
            target_lang: "English".to_string(),
            privacy_mode: false,
            translation_mode: "manual".to_string(),
            persona: String::new(),
        }
    }
}
```

Add test at the bottom of `src-tauri/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.target_lang, "English");
        assert_eq!(config.selected_model_index, 0);
        assert!(!config.privacy_mode);
        assert_eq!(config.translation_mode, "manual");
        assert!(config.models.is_empty());
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let config = AppConfig {
            models: vec![ModelConfig {
                api_key: "sk-test".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                display_name: "GPT-4o Mini".to_string(),
                temperature: Some(0.3),
                max_tokens: Some(1024),
            }],
            selected_model_index: 0,
            target_lang: "Japanese".to_string(),
            privacy_mode: true,
            translation_mode: "realtime".to_string(),
            persona: "formal".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.models[0].api_key, "sk-test");
        assert_eq!(deserialized.target_lang, "Japanese");
        assert!(deserialized.privacy_mode);
    }

    #[test]
    fn test_config_deserialize_missing_optional_fields() {
        let json = r#"{
            "models": [],
            "selected_model_index": 0,
            "target_lang": "English",
            "privacy_mode": false,
            "translation_mode": "manual",
            "persona": ""
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.translation_mode, "manual");
    }
}
```

- [ ] **Step 6: Add serde and serde_json to Cargo.toml dependencies**

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 7: Register config module in lib.rs**

Replace `src-tauri/src/lib.rs`:

```rust
pub mod config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 8: Run Rust tests to verify they pass**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo test
```

Expected: 3 tests pass (test_config_default, test_config_serialize_roundtrip, test_config_deserialize_missing_optional_fields).

- [ ] **Step 9: Verify transparent window**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected: A borderless, transparent, always-on-top window appears centered on screen, showing only the Vue content. No title bar, no window chrome.

- [ ] **Step 10: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: configure main window as transparent borderless overlay with config schema"
```

---

## Phase 2: Frontend — Floating Input & Settings UI

### Task 3: Build Floating Input View

**Files:**
- Modify: `realtime-translator-tauri/src/views/FloatingInput.vue`
- Modify: `realtime-translator-tauri/src/style.css`

- [ ] **Step 1: Build the FloatingInput component**

Replace `src/views/FloatingInput.vue`:

```vue
<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const textareaRef = ref<HTMLTextAreaElement | null>(null);

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleTranslate();
  }
  if (e.key === "Escape") {
    handleHide();
  }
}

async function handleTranslate() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;

  errorMessage.value = "";
  translatedText.value = "";
  isLoading.value = true;

  try {
    // Placeholder — will be replaced with actual LLM call in Task 10
    translatedText.value = `[TODO] Translation of: ${text}`;
  } catch (err) {
    errorMessage.value = String(err);
  } finally {
    isLoading.value = false;
  }
}

async function handleHide() {
  // Will be wired to Tauri invoke in Task 7
  console.log("hide window");
}

async function handleOpenSettings() {
  // Will be wired to Tauri invoke in Task 7
  console.log("open settings");
}

function clearAll() {
  inputText.value = "";
  translatedText.value = "";
  errorMessage.value = "";
}

onMounted(() => {
  nextTick(() => {
    textareaRef.value?.focus();
  });
});

defineExpose({ clearAll });
</script>

<template>
  <div
    class="w-full h-full flex items-center justify-center bg-black/60 backdrop-blur-sm rounded-xl"
  >
    <div class="w-full max-w-[580px] px-4 py-3 flex flex-col gap-2">
      <!-- Input area -->
      <div class="flex items-end gap-2">
        <textarea
          ref="textareaRef"
          v-model="inputText"
          @keydown="handleKeydown"
          placeholder="Type text to translate... (Enter to translate, Esc to hide)"
          rows="1"
          class="flex-1 resize-none bg-white/10 text-white placeholder:text-white/40
                 rounded-lg px-3 py-2 text-sm outline-none border border-white/20
                 focus:border-white/40 max-h-[120px] overflow-y-auto"
          style="field-sizing: content"
        />
        <button
          @click="handleOpenSettings"
          class="shrink-0 w-8 h-8 flex items-center justify-center
                 text-white/50 hover:text-white/80 transition-colors"
          title="Settings"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1
              0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0
              0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2
              2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65
              1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2
              0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68
              15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2
              0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65
              0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2
              0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9
              4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2
              2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65
              1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0
              2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0
              19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2
              2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
        </button>
      </div>

      <!-- Status area -->
      <div v-if="isLoading" class="text-white/60 text-xs">Translating...</div>
      <div v-if="errorMessage" class="text-red-400 text-xs">
        {{ errorMessage }}
      </div>
      <div
        v-if="translatedText"
        class="text-white/90 text-sm bg-white/5 rounded-lg px-3 py-2"
      >
        {{ translatedText }}
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify UI renders**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected: Transparent dark overlay with a text input area, a gear icon button, and no title bar. Text can be typed into the textarea.

- [ ] **Step 3: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add src/views/FloatingInput.vue
git commit -m "feat: floating input view with textarea, status display, and settings button"
```

---

### Task 4: Build Settings View

**Files:**
- Modify: `realtime-translator-tauri/src/views/Settings.vue`

- [ ] **Step 1: Build the Settings component**

Replace `src/views/Settings.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted } from "vue";

interface ModelConfig {
  api_key: string;
  base_url: string;
  model: string;
  display_name: string;
  temperature: number | null;
  max_tokens: number | null;
}

interface AppConfig {
  models: ModelConfig[];
  selected_model_index: number;
  target_lang: string;
  privacy_mode: boolean;
  translation_mode: string;
  persona: string;
}

const config = ref<AppConfig>({
  models: [],
  selected_model_index: 0,
  target_lang: "English",
  privacy_mode: false,
  translation_mode: "manual",
  persona: "",
});

const statusMessage = ref("");

const targetLanguages = [
  "English",
  "Chinese",
  "Japanese",
  "Korean",
  "French",
  "German",
  "Spanish",
  "Russian",
];

async function loadConfig() {
  // Will be wired to Tauri invoke in Task 9
  console.log("load config");
}

async function saveConfig() {
  statusMessage.value = "";
  try {
    // Will be wired to Tauri invoke in Task 9
    console.log("save config", config.value);
    statusMessage.value = "Saved!";
    setTimeout(() => (statusMessage.value = ""), 2000);
  } catch (err) {
    statusMessage.value = `Error: ${err}`;
  }
}

function addModel() {
  config.value.models.push({
    api_key: "",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    display_name: "",
    temperature: 0.3,
    max_tokens: 1024,
  });
}

function removeModel(index: number) {
  config.value.models.splice(index, 1);
  if (
    config.value.selected_model_index >= config.value.models.length
  ) {
    config.value.selected_model_index = Math.max(
      0,
      config.value.models.length - 1
    );
  }
}

onMounted(() => {
  loadConfig();
});
</script>

<template>
  <div class="min-h-screen bg-gray-900 text-white p-6">
    <div class="max-w-2xl mx-auto">
      <h1 class="text-xl font-bold mb-6">Settings</h1>

      <!-- Target Language -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">
          Target Language
        </h2>
        <select
          v-model="config.target_lang"
          class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2
                 text-sm focus:border-blue-500 outline-none"
        >
          <option v-for="lang in targetLanguages" :key="lang" :value="lang">
            {{ lang }}
          </option>
        </select>
      </section>

      <!-- Translation Mode -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">
          Translation Mode
        </h2>
        <div class="flex gap-4">
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              v-model="config.translation_mode"
              value="manual"
              class="accent-blue-500"
            />
            Manual (Enter to translate)
          </label>
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              v-model="config.translation_mode"
              value="realtime"
              class="accent-blue-500"
            />
            Realtime (auto after debounce)
          </label>
        </div>
      </section>

      <!-- Privacy Mode -->
      <section class="mb-6">
        <label class="flex items-center gap-3 text-sm cursor-pointer">
          <input
            type="checkbox"
            v-model="config.privacy_mode"
            class="accent-blue-500 w-4 h-4"
          />
          Privacy Mode (use local AIFW service)
        </label>
      </section>

      <!-- Models -->
      <section class="mb-6">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold text-white/70">Models</h2>
          <button
            @click="addModel"
            class="text-xs bg-blue-600 hover:bg-blue-500 px-3 py-1
                   rounded transition-colors"
          >
            + Add Model
          </button>
        </div>

        <div
          v-for="(model, index) in config.models"
          :key="index"
          class="bg-gray-800 border border-gray-700 rounded-lg p-4 mb-3"
        >
          <div class="flex items-center justify-between mb-3">
            <label class="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="radio"
                v-model="config.selected_model_index"
                :value="index"
                class="accent-blue-500"
              />
              <span class="text-white/60">Active</span>
            </label>
            <button
              @click="removeModel(index)"
              class="text-xs text-red-400 hover:text-red-300 transition-colors"
            >
              Remove
            </button>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="text-xs text-white/50 mb-1 block"
                >Display Name</label
              >
              <input
                v-model="model.display_name"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
            <div>
              <label class="text-xs text-white/50 mb-1 block">Model</label>
              <input
                v-model="model.model"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
            <div class="col-span-2">
              <label class="text-xs text-white/50 mb-1 block">Base URL</label>
              <input
                v-model="model.base_url"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
            <div class="col-span-2">
              <label class="text-xs text-white/50 mb-1 block">API Key</label>
              <input
                v-model="model.api_key"
                type="password"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
          </div>
        </div>

        <div
          v-if="config.models.length === 0"
          class="text-white/40 text-sm text-center py-4"
        >
          No models configured. Click "+ Add Model" to add one.
        </div>
      </section>

      <!-- Persona -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">
          Translation Persona (optional)
        </h2>
        <input
          v-model="config.persona"
          placeholder="e.g. formal, casual, technical..."
          class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2
                 text-sm focus:border-blue-500 outline-none"
        />
      </section>

      <!-- Save -->
      <div class="flex items-center gap-3">
        <button
          @click="saveConfig"
          class="bg-blue-600 hover:bg-blue-500 px-6 py-2 rounded-lg
                 text-sm font-medium transition-colors"
        >
          Save
        </button>
        <span
          v-if="statusMessage"
          :class="
            statusMessage.startsWith('Error')
              ? 'text-red-400'
              : 'text-green-400'
          "
          class="text-sm"
        >
          {{ statusMessage }}
        </span>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify settings UI renders**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected: Main window shows the floating input. Navigate to `#/settings` to see the settings form with all sections (language, mode, privacy, models, persona, save button).

- [ ] **Step 3: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add src/views/Settings.vue
git commit -m "feat: settings view with model management, language, mode, and persona config"
```

---

### Task 5: Implement Config Store

**Files:**
- Create: `realtime-translator-tauri/src/stores/config.ts`

- [ ] **Step 1: Write the config store with Tauri IPC calls**

Create `src/stores/config.ts`:

```typescript
import { reactive, toRaw } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ModelConfig {
  api_key: string;
  base_url: string;
  model: string;
  display_name: string;
  temperature: number | null;
  max_tokens: number | null;
}

export interface AppConfig {
  models: ModelConfig[];
  selected_model_index: number;
  target_lang: string;
  privacy_mode: boolean;
  translation_mode: string;
  persona: string;
}

const defaultConfig: AppConfig = {
  models: [],
  selected_model_index: 0,
  target_lang: "English",
  privacy_mode: false,
  translation_mode: "manual",
  persona: "",
};

export const appConfig = reactive<AppConfig>({ ...defaultConfig });

export async function loadConfig(): Promise<void> {
  try {
    const loaded = await invoke<AppConfig>("read_config");
    Object.assign(appConfig, loaded);
  } catch {
    // Config file doesn't exist yet — use defaults
    Object.assign(appConfig, { ...defaultConfig });
  }
}

export async function saveConfig(): Promise<void> {
  await invoke("save_config", { config: toRaw(appConfig) });
}

export function getActiveModel(): ModelConfig | null {
  if (
    appConfig.models.length === 0 ||
    appConfig.selected_model_index >= appConfig.models.length
  ) {
    return null;
  }
  return appConfig.models[appConfig.selected_model_index];
}
```

- [ ] **Step 2: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add src/stores/config.ts
git commit -m "feat: reactive config store with Tauri IPC load/save"
```

---

## Phase 3: Rust Backend — Shortcuts, Window, Clipboard

### Task 6: Implement Window Management Commands

**Files:**
- Create: `realtime-translator-tauri/src-tauri/src/commands/mod.rs`
- Create: `realtime-translator-tauri/src-tauri/src/commands/window.rs`
- Modify: `realtime-translator-tauri/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the commands module structure**

Create `src-tauri/src/commands/mod.rs`:

```rust
pub mod window;
```

Create `src-tauri/src/commands/window.rs`:

```rust
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("index.html#/settings".into()),
    )
    .title("Settings")
    .inner_size(640.0, 520.0)
    .decorations(true)
    .resizable(true)
    .always_on_top(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}
```

- [ ] **Step 2: Register commands in lib.rs**

Replace `src-tauri/src/lib.rs`:

```rust
pub mod commands;
pub mod config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Build to verify compilation**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo build
```

Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add src-tauri/src/commands/ src-tauri/src/lib.rs
git commit -m "feat: window management commands (hide, show, open settings)"
```

---

### Task 7: Implement Global Shortcut

**Files:**
- Create: `realtime-translator-tauri/src-tauri/src/shortcut.rs`
- Modify: `realtime-translator-tauri/src-tauri/Cargo.toml`
- Modify: `realtime-translator-tauri/src-tauri/src/lib.rs`
- Modify: `realtime-translator-tauri/src-tauri/capabilities/default.json`
- Modify: `realtime-translator-tauri/src/composables/useTauriEvents.ts`

- [ ] **Step 1: Add global-shortcut plugin dependency**

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
tauri-plugin-global-shortcut = "2"
```

- [ ] **Step 2: Write shortcut registration module**

Create `src-tauri/src/shortcut.rs`:

```rust
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

pub fn register(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyY);

    let app_handle = app.clone();
    app.global_shortcut().on_shortcut(shortcut, move |_app, _event, state| {
        if state == ShortcutState::Pressed {
            let main_window = app_handle
                .get_webview_window("main")
                .expect("main window not found");

            if main_window.is_visible().unwrap_or(false) {
                let _ = main_window.hide();
            } else {
                let _ = main_window.show();
                let _ = main_window.set_focus();
                let _ = main_window.emit("shortcut-triggered", ());
            }
        }
    })?;

    Ok(())
}
```

- [ ] **Step 3: Wire shortcut registration in lib.rs**

Replace `src-tauri/src/lib.rs`:

```rust
pub mod commands;
pub mod config;
pub mod shortcut;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            shortcut::register(&handle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Add shortcut plugin permission**

Update `src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "description": "Default capabilities for the translator app",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-center",
    "core:window:allow-set-always-on-top",
    "core:window:allow-create",
    "core:window:allow-get-all-windows",
    "core:window:allow-is-visible",
    "global-shortcut:allow-register",
    "global-shortcut:allow-on-shortcut",
    "shell:default"
  ]
}
```

- [ ] **Step 5: Create Tauri events composable**

Create `src/composables/useTauriEvents.ts`:

```typescript
import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function useShortcutTriggered(callback: () => void) {
  let unlisten: UnlistenFn | null = null;

  onMounted(async () => {
    unlisten = await listen("shortcut-triggered", () => {
      callback();
    });
  });

  onUnmounted(() => {
    unlisten?.();
  });
}
```

- [ ] **Step 6: Wire shortcut event into FloatingInput.vue**

Update `src/views/FloatingInput.vue` — add import and usage in the `<script setup>` block:

Add after the existing imports:

```typescript
import { useShortcutTriggered } from "../composables/useTauriEvents";
```

Add after the `defineExpose` line:

```typescript
useShortcutTriggered(() => {
  clearAll();
  nextTick(() => {
    textareaRef.value?.focus();
  });
});
```

- [ ] **Step 7: Build to verify compilation**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo build
```

Expected: Compiles without errors.

- [ ] **Step 8: Test global shortcut**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected: Window starts hidden. Press `Alt+Y` — window appears and input is focused. Press `Alt+Y` again — window hides. Repeat several times to verify reliability.

- [ ] **Step 9: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: global shortcut Alt+Y with toggle visibility and event emission"
```

---

### Task 8: Implement Clipboard & Paste Simulation

**Files:**
- Create: `realtime-translator-tauri/src-tauri/src/commands/clipboard.rs`
- Modify: `realtime-translator-tauri/src-tauri/Cargo.toml`
- Modify: `realtime-translator-tauri/src-tauri/src/lib.rs`
- Modify: `realtime-translator-tauri/src-tauri/capabilities/default.json`

- [ ] **Step 1: Add enigo dependency**

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
enigo = { version = "0.2", features = ["serde"] }
```

- [ ] **Step 2: Write a test for the paste simulation logic**

Create `src-tauri/src/commands/clipboard.rs`:

```rust
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

fn platform_paste_key() -> Key {
    if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    }
}

/// Simulates a Ctrl+V (or Cmd+V on macOS) paste.
fn simulate_paste_keystrokes(enigo: &mut Enigo) -> Result<(), String> {
    let modifier = platform_paste_key();
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| format!("key press: {e}"))?;
    enigo
        .key(Key::V, Direction::Click)
        .map_err(|e| format!("v click: {e}"))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| format!("key release: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn simulate_paste(text: String) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to init enigo: {e}"))?;

    // Backup current clipboard
    let backup = arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .unwrap_or_default();

    // Write translation to clipboard
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("clipboard init: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("clipboard set: {e}"))?;
    }

    // Small delay to ensure clipboard is set
    thread::sleep(Duration::from_millis(50));

    // Simulate paste
    simulate_paste_keystrokes(&mut enigo)?;

    // Wait for paste to complete, then restore clipboard
    thread::sleep(Duration::from_millis(100));
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(backup);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_paste_key() {
        let key = platform_paste_key();
        if cfg!(target_os = "macos") {
            assert!(matches!(key, Key::Meta));
        } else {
            assert!(matches!(key, Key::Control));
        }
    }
}
```

- [ ] **Step 3: Add arboard dependency to Cargo.toml**

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
arboard = "3"
```

- [ ] **Step 4: Register clipboard command in lib.rs**

Update the `invoke_handler` in `src-tauri/src/lib.rs` to include:

```rust
commands::clipboard::simulate_paste,
```

- [ ] **Step 5: Add clipboard module to commands/mod.rs**

Update `src-tauri/src/commands/mod.rs`:

```rust
pub mod clipboard;
pub mod window;
```

- [ ] **Step 6: Run Rust tests**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo test
```

Expected: All tests pass, including `test_platform_paste_key`.

- [ ] **Step 7: Build to verify compilation**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo build
```

Expected: Compiles without errors.

- [ ] **Step 8: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: clipboard backup + paste simulation with enigo and arboard"
```

---

### Task 9: Implement Config Read/Write Commands

**Files:**
- Create: `realtime-translator-tauri/src-tauri/src/commands/config_cmd.rs`
- Modify: `realtime-translator-tauri/src-tauri/src/commands/mod.rs`
- Modify: `realtime-translator-tauri/src-tauri/src/lib.rs`
- Modify: `realtime-translator-tauri/src-tauri/Cargo.toml`
- Modify: `realtime-translator-tauri/src-tauri/capabilities/default.json`
- Modify: `realtime-translator-tauri/src/views/Settings.vue`
- Modify: `realtime-translator-tauri/src/views/FloatingInput.vue`

- [ ] **Step 1: Write tests for config file I/O**

Create `src-tauri/src/commands/config_cmd.rs`:

```rust
use crate::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("config.json"))
}

#[tauri::command]
pub fn read_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_roundtrip_via_json() {
        let config = AppConfig {
            models: vec![crate::config::ModelConfig {
                api_key: "sk-test123".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                display_name: "Test Model".to_string(),
                temperature: Some(0.3),
                max_tokens: Some(1024),
            }],
            selected_model_index: 0,
            target_lang: "Japanese".to_string(),
            privacy_mode: false,
            translation_mode: "manual".to_string(),
            persona: "formal".to_string(),
        };

        // Serialize to JSON string
        let json = serde_json::to_string_pretty(&config).unwrap();

        // Write to a temp file
        let dir = std::env::temp_dir().join("translator_test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        // Read back and deserialize
        let content = fs::read_to_string(&path).unwrap();
        let loaded: AppConfig = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.models.len(), 1);
        assert_eq!(loaded.models[0].api_key, "sk-test123");
        assert_eq!(loaded.target_lang, "Japanese");
        assert_eq!(loaded.persona, "formal");

        // Cleanup
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_empty_config_returns_default() {
        let json = r#"{"models":[],"selected_model_index":0,"target_lang":"English","privacy_mode":false,"translation_mode":"manual","persona":""}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.models.is_empty());
        assert_eq!(config.target_lang, "English");
    }
}
```

- [ ] **Step 2: Register module**

Update `src-tauri/src/commands/mod.rs`:

```rust
pub mod clipboard;
pub mod config_cmd;
pub mod window;
```

- [ ] **Step 3: Register commands in lib.rs**

Update `src-tauri/src/lib.rs` — add to invoke_handler:

```rust
commands::config_cmd::read_config,
commands::config_cmd::save_config,
```

- [ ] **Step 4: Add tauri-plugin-fs dependency for path resolution**

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
tauri-plugin-fs = "2"
```

Register in lib.rs (add before `.invoke_handler`):

```rust
.plugin(tauri_plugin_fs::init())
```

- [ ] **Step 5: Add fs plugin permissions**

Update `src-tauri/capabilities/default.json` — add to permissions:

```json
"fs:default",
"fs:allow-app-read",
"fs:allow-app-write"
```

- [ ] **Step 6: Run Rust tests**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo test
```

Expected: All tests pass (5 total: 3 from config, 1 from clipboard, 2 from config_cmd).

- [ ] **Step 7: Wire Settings.vue to use real config store**

Replace the `<script setup>` block in `src/views/Settings.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted } from "vue";
import { appConfig, loadConfig, saveConfig as persistConfig } from "../stores/config";

const statusMessage = ref("");

const targetLanguages = [
  "English",
  "Chinese",
  "Japanese",
  "Korean",
  "French",
  "German",
  "Spanish",
  "Russian",
];

async function load() {
  try {
    await loadConfig();
  } catch (err) {
    console.error("Failed to load config:", err);
  }
}

async function saveConfig() {
  statusMessage.value = "";
  try {
    await persistConfig();
    statusMessage.value = "Saved!";
    setTimeout(() => (statusMessage.value = ""), 2000);
  } catch (err) {
    statusMessage.value = `Error: ${err}`;
  }
}

function addModel() {
  appConfig.models.push({
    api_key: "",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    display_name: "",
    temperature: 0.3,
    max_tokens: 1024,
  });
}

function removeModel(index: number) {
  appConfig.models.splice(index, 1);
  if (appConfig.selected_model_index >= appConfig.models.length) {
    appConfig.selected_model_index = Math.max(0, appConfig.models.length - 1);
  }
}

onMounted(() => {
  load();
});
</script>
```

Replace the `<template>` block to bind to `appConfig` instead of local `config`:

```vue
<template>
  <div class="min-h-screen bg-gray-900 text-white p-6">
    <div class="max-w-2xl mx-auto">
      <h1 class="text-xl font-bold mb-6">Settings</h1>

      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">Target Language</h2>
        <select
          v-model="appConfig.target_lang"
          class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:border-blue-500 outline-none"
        >
          <option v-for="lang in targetLanguages" :key="lang" :value="lang">{{ lang }}</option>
        </select>
      </section>

      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">Translation Mode</h2>
        <div class="flex gap-4">
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input type="radio" v-model="appConfig.translation_mode" value="manual" class="accent-blue-500" />
            Manual (Enter to translate)
          </label>
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input type="radio" v-model="appConfig.translation_mode" value="realtime" class="accent-blue-500" />
            Realtime (auto after debounce)
          </label>
        </div>
      </section>

      <section class="mb-6">
        <label class="flex items-center gap-3 text-sm cursor-pointer">
          <input type="checkbox" v-model="appConfig.privacy_mode" class="accent-blue-500 w-4 h-4" />
          Privacy Mode (use local AIFW service)
        </label>
      </section>

      <section class="mb-6">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold text-white/70">Models</h2>
          <button @click="addModel" class="text-xs bg-blue-600 hover:bg-blue-500 px-3 py-1 rounded transition-colors">
            + Add Model
          </button>
        </div>

        <div v-for="(model, index) in appConfig.models" :key="index" class="bg-gray-800 border border-gray-700 rounded-lg p-4 mb-3">
          <div class="flex items-center justify-between mb-3">
            <label class="flex items-center gap-2 text-sm cursor-pointer">
              <input type="radio" v-model="appConfig.selected_model_index" :value="index" class="accent-blue-500" />
              <span class="text-white/60">Active</span>
            </label>
            <button @click="removeModel(index)" class="text-xs text-red-400 hover:text-red-300 transition-colors">Remove</button>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="text-xs text-white/50 mb-1 block">Display Name</label>
              <input v-model="model.display_name" class="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1.5 text-sm focus:border-blue-500 outline-none" />
            </div>
            <div>
              <label class="text-xs text-white/50 mb-1 block">Model</label>
              <input v-model="model.model" class="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1.5 text-sm focus:border-blue-500 outline-none" />
            </div>
            <div class="col-span-2">
              <label class="text-xs text-white/50 mb-1 block">Base URL</label>
              <input v-model="model.base_url" class="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1.5 text-sm focus:border-blue-500 outline-none" />
            </div>
            <div class="col-span-2">
              <label class="text-xs text-white/50 mb-1 block">API Key</label>
              <input v-model="model.api_key" type="password" class="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1.5 text-sm focus:border-blue-500 outline-none" />
            </div>
          </div>
        </div>

        <div v-if="appConfig.models.length === 0" class="text-white/40 text-sm text-center py-4">
          No models configured. Click "+ Add Model" to add one.
        </div>
      </section>

      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">Translation Persona (optional)</h2>
        <input v-model="appConfig.persona" placeholder="e.g. formal, casual, technical..." class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:border-blue-500 outline-none" />
      </section>

      <div class="flex items-center gap-3">
        <button @click="saveConfig" class="bg-blue-600 hover:bg-blue-500 px-6 py-2 rounded-lg text-sm font-medium transition-colors">
          Save
        </button>
        <span v-if="statusMessage" :class="statusMessage.startsWith('Error') ? 'text-red-400' : 'text-green-400'" class="text-sm">
          {{ statusMessage }}
        </span>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 8: Wire FloatingInput.vue window commands**

Update `src/views/FloatingInput.vue` — add import and update handlers in `<script setup>`:

Add after existing imports:

```typescript
import { invoke } from "@tauri-apps/api/core";
import { appConfig, loadConfig, getActiveModel } from "../stores/config";
```

Replace the `handleHide` function:

```typescript
async function handleHide() {
  await invoke("hide_main_window");
}
```

Replace the `handleOpenSettings` function:

```typescript
async function handleOpenSettings() {
  await invoke("open_settings_window");
}
```

- [ ] **Step 9: End-to-end test**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected behavior:
1. Main window appears (hidden initially, press Alt+Y to show)
2. Click gear icon → settings window opens
3. Add a model, set language, click Save → "Saved!" message
4. Close and reopen app → settings persist
5. Press Esc or Alt+Y to hide main window

- [ ] **Step 10: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: config read/write commands with persistent settings via Tauri fs"
```

---

## Phase 4: LLM Translation Client

### Task 10: Implement LLM Client Service

**Files:**
- Create: `realtime-translator-tauri/src/services/llm-client.ts`

- [ ] **Step 1: Write the LLM client service**

Create `src/services/llm-client.ts`:

```typescript
import { getActiveModel, appConfig } from "../stores/config";

const TRANSLATION_SYSTEM_PROMPT = `You are a translation engine. Translate the user's input text to the target language.
Rules:
- Output ONLY the translated text, nothing else.
- Preserve the original punctuation style and line breaks.
- Do not add explanations, notes, or any extra content.
- If the input is already in the target language, output it as-is.`;

const AIFW_DEFAULT_URL = "http://localhost:8844/api/call";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
}

interface ChatCompletionResponse {
  choices: Array<{
    message: {
      content: string;
    };
  }>;
}

export async function translate(text: string): Promise<string> {
  const model = getActiveModel();
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }

  const systemPrompt = buildSystemPrompt();

  const messages: ChatMessage[] = [
    { role: "system", content: systemPrompt },
    { role: "user", content: text },
  ];

  const body: ChatCompletionRequest = {
    model: model.model,
    messages,
    temperature: model.temperature ?? 0.3,
    max_tokens: model.max_tokens ?? 1024,
  };

  const url = getRequestUrl(model.base_url);

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (!appConfig.privacy_mode && model.api_key) {
    headers["Authorization"] = `Bearer ${model.api_key}`;
  }

  const response = await fetch(`${url}/chat/completions`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API error ${response.status}: ${errorText}`);
  }

  const data = (await response.json()) as ChatCompletionResponse;

  if (!data.choices || data.choices.length === 0) {
    throw new Error("Empty response from LLM API");
  }

  return data.choices[0].message.content.trim();
}

function buildSystemPrompt(): string {
  let prompt = TRANSLATION_SYSTEM_PROMPT;
  prompt += `\nTarget language: ${appConfig.target_lang}.`;

  if (appConfig.persona) {
    prompt += `\nTranslation style: ${appConfig.persona}.`;
  }

  return prompt;
}

function getRequestUrl(baseUrl: string): string {
  if (appConfig.privacy_mode) {
    return AIFW_DEFAULT_URL;
  }

  // Strip trailing /v1 if present — we append /chat/completions ourselves
  const clean = baseUrl.replace(/\/v1\/?$/, "").replace(/\/$/, "");
  return clean;
}

// For testing: expose internals
export const _internals = {
  buildSystemPrompt,
  getRequestUrl,
  TRANSLATION_SYSTEM_PROMPT,
  AIFW_DEFAULT_URL,
};
```

- [ ] **Step 2: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add src/services/llm-client.ts
git commit -m "feat: LLM client service with OpenAI-compatible API and AIFW routing"
```

---

### Task 11: Wire Translation into Floating Input

**Files:**
- Modify: `realtime-translator-tauri/src/views/FloatingInput.vue`

- [ ] **Step 1: Replace placeholder translation with real LLM call**

Update `src/views/FloatingInput.vue` — replace the `<script setup>` block:

```vue
<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { translate } from "../services/llm-client";
import { loadConfig } from "../stores/config";
import { useShortcutTriggered } from "../composables/useTauriEvents";

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const textareaRef = ref<HTMLTextAreaElement | null>(null);

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleTranslate();
  }
  if (e.key === "Escape") {
    handleHide();
  }
}

async function handleTranslate() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;

  errorMessage.value = "";
  translatedText.value = "";
  isLoading.value = true;

  try {
    const result = await translate(text);
    translatedText.value = result;

    // Hide window and paste
    await invoke("hide_main_window");
    await invoke("simulate_paste", { text: result });
  } catch (err) {
    errorMessage.value = String(err);
  } finally {
    isLoading.value = false;
  }
}

async function handleHide() {
  await invoke("hide_main_window");
}

async function handleOpenSettings() {
  await invoke("open_settings_window");
}

function clearAll() {
  inputText.value = "";
  translatedText.value = "";
  errorMessage.value = "";
}

onMounted(async () => {
  await loadConfig();
  nextTick(() => {
    textareaRef.value?.focus();
  });
});

defineExpose({ clearAll });

useShortcutTriggered(() => {
  clearAll();
  nextTick(() => {
    textareaRef.value?.focus();
  });
});
</script>
```

Keep the `<template>` block from Task 3 unchanged.

- [ ] **Step 2: Add clipboard permission**

Update `src-tauri/capabilities/default.json` — ensure the `clipboard` permission is present. If using `tauri-plugin-clipboard-manager`, add:

```json
"clipboard-manager:allow-write-text",
"clipboard-manager:allow-read-text"
```

(If using `arboard` directly in Rust without the Tauri plugin, no additional permission is needed — the `simulate_paste` command handles it.)

- [ ] **Step 3: End-to-end translation test**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected behavior:
1. Press Alt+Y → floating window appears
2. Click gear → add a model with valid API key and base URL → Save
3. Type text (e.g. "你好世界") → press Enter
4. "Translating..." appears → translated text appears → window hides → text is pasted into whatever app has focus
5. If no model configured → error message "No model configured"

- [ ] **Step 4: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: full translation flow — input → LLM → paste injection"
```

---

## Phase 5: AIFW Integration

### Task 12: Implement AIFW Process Controller

**Files:**
- Create: `realtime-translator-tauri/src-tauri/src/aifw.rs`
- Modify: `realtime-translator-tauri/src-tauri/src/lib.rs`
- Modify: `realtime-translator-tauri/src-tauri/src/commands/mod.rs`
- Modify: `realtime-translator-tauri/src-tauri/Cargo.toml`

- [ ] **Step 1: Write AIFW process controller**

Create `src-tauri/src/aifw.rs`:

```rust
use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::AppHandle;

pub struct AifwState {
    child: Mutex<Option<Child>>,
}

impl AifwState {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    *guard = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn start(&self, exe_path: &str) -> Result<(), String> {
        if self.is_running() {
            return Ok(());
        }

        let child = Command::new(exe_path)
            .spawn()
            .map_err(|e| format!("Failed to start AIFW: {e}"))?;

        let mut guard = self.child.lock().unwrap();
        *guard = Some(child);
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *guard {
            child
                .kill()
                .map_err(|e| format!("Failed to kill AIFW: {e}"))?;
            *guard = None;
        }
        Ok(())
    }
}

#[tauri::command]
pub fn start_aifw(app: AppHandle, exe_path: String) -> Result<(), String> {
    let state = app.state::<AifwState>();
    state.start(&exe_path)
}

#[tauri::command]
pub fn stop_aifw(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AifwState>();
    state.stop()
}

#[tauri::command]
pub fn aifw_status(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<AifwState>();
    Ok(state.is_running())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aifw_state_initial_not_running() {
        let state = AifwState::new();
        assert!(!state.is_running());
    }

    #[test]
    fn test_aifw_stop_when_not_running() {
        let state = AifwState::new();
        assert!(state.stop().is_ok());
    }

    #[test]
    fn test_aifw_start_nonexistent_exe() {
        let state = AifwState::new();
        let result = state.start("nonexistent_aifw_server.exe");
        assert!(result.is_err());
        assert!(!state.is_running());
    }
}
```

- [ ] **Step 2: Register AIFW state and commands**

Update `src-tauri/src/commands/mod.rs`:

```rust
pub mod clipboard;
pub mod config_cmd;
pub mod window;
```

Update `src-tauri/src/lib.rs`:

```rust
pub mod aifw;
pub mod commands;
pub mod config;
pub mod shortcut;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_fs::init())
        .manage(aifw::AifwState::new())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
            commands::clipboard::simulate_paste,
            commands::config_cmd::read_config,
            commands::config_cmd::save_config,
            aifw::start_aifw,
            aifw::stop_aifw,
            aifw::aifw_status,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            shortcut::register(&handle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Run all Rust tests**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo test
```

Expected: All 8 tests pass.

- [ ] **Step 4: Build to verify compilation**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri\src-tauri
cargo build
```

Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: AIFW subprocess lifecycle management (start, stop, status)"
```

---

### Task 13: Wire AIFW Start/Stop in Settings UI

**Files:**
- Modify: `realtime-translator-tauri/src/views/Settings.vue`
- Modify: `realtime-translator-tauri/src-tauri/capabilities/default.json`

- [ ] **Step 1: Add AIFW controls to Settings view**

Add to the `<script setup>` block in `src/views/Settings.vue` (after existing imports and before `load`):

```typescript
import { ref as vueRef } from "vue";
import { invoke } from "@tauri-apps/api/core";

const aifwExePath = vueRef("aifw_server.exe");
const aifwRunning = vueRef(false);

async function checkAifwStatus() {
  try {
    aifwRunning.value = await invoke<boolean>("aifw_status");
  } catch {
    aifwRunning.value = false;
  }
}

async function startAifw() {
  try {
    await invoke("start_aifw", { exePath: aifwExePath.value });
    aifwRunning.value = true;
  } catch (err) {
    statusMessage.value = `AIFW Error: ${err}`;
  }
}

async function stopAifw() {
  try {
    await invoke("stop_aifw");
    aifwRunning.value = false;
  } catch (err) {
    statusMessage.value = `AIFW Error: ${err}`;
  }
}
```

Update `onMounted` to also call `checkAifwStatus()`.

- [ ] **Step 2: Add AIFW section to Settings template**

Add before the Save button section in the Settings template:

```html
<!-- AIFW Control -->
<section class="mb-6">
  <h2 class="text-sm font-semibold text-white/70 mb-2">AIFW Service (Privacy Mode)</h2>
  <div class="flex items-center gap-3">
    <input
      v-model="aifwExePath"
      placeholder="Path to aifw_server.exe"
      class="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:border-blue-500 outline-none"
    />
    <button
      v-if="!aifwRunning"
      @click="startAifw"
      class="shrink-0 bg-green-600 hover:bg-green-500 px-4 py-2 rounded-lg text-sm transition-colors"
    >
      Start
    </button>
    <button
      v-else
      @click="stopAifw"
      class="shrink-0 bg-red-600 hover:bg-red-500 px-4 py-2 rounded-lg text-sm transition-colors"
    >
      Stop
    </button>
    <span :class="aifwRunning ? 'text-green-400' : 'text-white/40'" class="text-xs shrink-0">
      {{ aifwRunning ? 'Running' : 'Stopped' }}
    </span>
  </div>
</section>
```

- [ ] **Step 3: End-to-end AIFW test**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri dev
```

Expected behavior:
1. Open settings → AIFW section shows "Stopped"
2. Enter a path and click Start → if exe doesn't exist, shows error
3. If exe exists and starts → status shows "Running"
4. Click Stop → status shows "Stopped"
5. With Privacy Mode ON, translation requests go to localhost:8844

- [ ] **Step 4: Commit**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "feat: AIFW service control UI in settings with start/stop/status"
```

---

## Phase 6: Integration Testing & Polish

### Task 14: Full Integration Smoke Test

**Files:**
- No new files — manual testing checklist

- [ ] **Step 1: Build release and verify**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
npm run tauri build
```

Expected: Build succeeds. Binary at `src-tauri/target/release/realtime-translator.exe`.

- [ ] **Step 2: Smoke test — complete translation flow**

1. Launch the built executable
2. Press `Alt+Y` → floating window appears centered, transparent, no title bar
3. Click gear icon → settings window opens
4. Add a model (e.g. base_url: `https://api.openai.com/v1`, api_key: your key, model: `gpt-4o-mini`)
5. Set target language to "Chinese"
6. Click Save → "Saved!"
7. Close settings, press `Alt+Y` if needed
8. Type "Hello, how are you?" → press Enter
9. "Translating..." shows → translated text appears → window hides → text is pasted into active window
10. Press `Alt+Y` again → input is clear, ready for new input
11. Press `Esc` → window hides
12. Test with Privacy Mode ON (if AIFW server is available)

- [ ] **Step 3: Smoke test — edge cases**

1. No models configured → shows error "No model configured"
2. Invalid API key → shows error from API
3. Empty input → Enter does nothing
4. Very long text → textarea scrolls, translation completes
5. Settings persist across app restarts

- [ ] **Step 4: Commit final state**

```powershell
cd C:\Users\Hong\Documents\Works_temp\prompit\realtime-translator-tauri
git add -A
git commit -m "chore: integration smoke test passed — MVP complete"
```

---

## Summary

| Phase | Tasks | Description |
|---|---|---|
| 1 | 1-2 | Tauri project init, transparent window, config schema |
| 2 | 3-5 | Floating input UI, settings UI, config store |
| 3 | 6-9 | Window commands, global shortcut, clipboard/paste, config I/O |
| 4 | 10-11 | LLM client, full translation flow wiring |
| 5 | 12-13 | AIFW process controller, AIFW UI |
| 6 | 14 | Integration smoke test |

**Total: 14 tasks, ~60 steps.** Each task produces a working, committable increment.
