# Sparkle Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Sparkle" (灵感) mode to Prompit that transforms user input via user-defined system prompts, with a built-in Polish sparkle and encrypted storage.

**Architecture:** New mode registered in the existing `MODES` array. Rust backend mirrors `persona.rs` for encrypted `sparkles.json` storage. Frontend adds `sparkleStore`, branches `translate()` by mode, and adds sparkle-specific UI in Settings and FloatingInput toolbar.

**Tech Stack:** Tauri v2 (Rust), Vue 3, Tailwind v4, TypeScript, @lucide/vue

---

## Task 1: Rust backend — sparkle encrypted storage

**Files:**
- Create: `src-tauri/src/commands/sparkle.rs`
- Modify: `src-tauri/src/commands/mod.rs:1-8`
- Modify: `src-tauri/src/lib.rs:101-133`

- [ ] **Step 1: Create `sparkle.rs`**

Copy the full content of `src-tauri/src/commands/persona.rs` and adapt it:

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkleEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct EncryptedSparkles {
    ciphertext: String,
    nonce: String,
}

fn sparkle_key() -> [u8; 32] {
    // Reuse the same key derivation as personas — same machine, same user
    use sha2::{Digest, Sha256};
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".into());
    let app_id = "com.translator.realtime";
    let seed = format!("{}:{}:{}", hostname, username, app_id);
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn sparkles_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("sparkles.json"))
}

fn load_sparkles_encrypted(app: &AppHandle) -> Result<Vec<SparkleEntry>, String> {
    let path = sparkles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let enc: EncryptedSparkles = serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let key = sparkle_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce_bytes = BASE64
        .decode(&enc.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = BASE64
        .decode(&enc.ciphertext)
        .map_err(|e| format!("decode ct: {e}"))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decrypt: {e}"))?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("deserialize: {e}"))
}

fn save_sparkles_encrypted(app: &AppHandle, sparkles: &[SparkleEntry]) -> Result<(), String> {
    let path = sparkles_path(app)?;
    let json = serde_json::to_vec(sparkles).map_err(|e| format!("serialize: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use rand::RngCore;

    let key = sparkle_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, json.as_slice())
        .map_err(|e| format!("encrypt: {e}"))?;

    let enc = EncryptedSparkles {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    };
    let out = serde_json::to_string_pretty(&enc).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn read_sparkles(app: AppHandle) -> Result<Vec<SparkleEntry>, String> {
    load_sparkles_encrypted(&app)
}

#[tauri::command]
pub fn save_sparkles(app: AppHandle, sparkles: Vec<SparkleEntry>) -> Result<(), String> {
    save_sparkles_encrypted(&app, &sparkles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkle_entry_serialize_roundtrip() {
        let entries = vec![
            SparkleEntry {
                name: "Polish".to_string(),
                prompt: "Polish the text".to_string(),
                enabled: true,
            },
        ];
        let json = serde_json::to_string(&entries).unwrap();
        let deserialized: Vec<SparkleEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Polish");
        assert!(deserialized[0].enabled);
    }
}
```

- [ ] **Step 2: Register module in `commands/mod.rs`**

Add this line to `src-tauri/src/commands/mod.rs`:

```rust
pub mod sparkle;
```

- [ ] **Step 3: Register commands in `lib.rs`**

In `src-tauri/src/lib.rs`, add two lines inside the `tauri::generate_handler![]` macro (after the `commands::persona::save_personas` line at ~126):

```rust
commands::sparkle::read_sparkles,
commands::sparkle::save_sparkles,
```

- [ ] **Step 4: Build and verify**

Run: `cd c:/Users/Hong/Documents/Works_temp/prompit && npx tauri build --debug 2>&1 | head -20`

If the build succeeds, proceed. If it fails, fix compilation errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/sparkle.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(sparkle): add Rust backend for encrypted sparkle storage"
```

---

## Task 2: Frontend config — sparkleStore and MODES registration

**Files:**
- Modify: `src/stores/config.ts`

- [ ] **Step 1: Add SparkleEntry interface and sparkleStore**

At the top of `src/stores/config.ts`, add the import for Sparkles icon (after the existing `Languages` import on line 5):

```ts
import { Languages, Sparkles } from "@lucide/vue";
```

Add `SparkleEntry` interface after `PersonaConfig` (around line 45):

```ts
export interface SparkleEntry {
  name: string;
  prompt: string;
  enabled: boolean;
}
```

Add `sparkleStore` after `personaStore` (after line 185):

```ts
export const sparkleStore = reactive<{ sparkles: SparkleEntry[] }>({
  sparkles: [],
});
```

- [ ] **Step 2: Add sparkle config fields to AppConfig**

In the `AppConfig` interface (around line 60-74), add after `history_limit`:

```ts
sparkle_active_provider_index: number;
sparkle_active_model_index: number;
```

In `defaultConfig` (around line 76-90), add after `history_limit: 50`:

```ts
sparkle_active_provider_index: 0,
sparkle_active_model_index: 0,
```

- [ ] **Step 3: Add sparkle to MODES array**

Change the `MODES` array (around line 393-400) to:

```ts
export const MODES: ModeDefinition[] = [
  {
    id: "translate",
    icon: Languages,
    labelKey: "modes.translate",
    settingTabKey: "translation",
  },
  {
    id: "sparkle",
    icon: Sparkles,
    labelKey: "modes.sparkle",
    settingTabKey: "sparkle",
  },
];
```

- [ ] **Step 4: Add loadSparkles and saveSparkles functions**

Add after the `loadConfig`/`saveConfig` pattern, before the `MODES` array:

```ts
export async function loadSparkles(): Promise<void> {
  try {
    const entries = await invoke<SparkleEntry[]>("read_sparkles");
    if (entries.length === 0) {
      // Seed the built-in Polish sparkle
      sparkleStore.sparkles = [
        {
          name: "润色 (Polish)",
          prompt: "Detect the language of the user's input. Adopt the role of a native speaker of that language. Rewrite the user's input as a more idiomatic, accurate, and natural expression in the same language, preserving the original meaning and intent.",
          enabled: true,
        },
      ];
      await saveSparkles();
    } else {
      sparkleStore.sparkles = entries;
    }
  } catch (err) {
    console.error("Failed to load sparkles:", err);
    sparkleStore.sparkles = [
      {
        name: "润色 (Polish)",
        prompt: "Detect the language of the user's input. Adopt the role of a native speaker of that language. Rewrite the user's input as a more idiomatic, accurate, and natural expression in the same language, preserving the original meaning and intent.",
        enabled: true,
      },
    ];
  }
}

export async function saveSparkles(): Promise<void> {
  try {
    await invoke("save_sparkles", { sparkles: sparkleStore.sparkles });
  } catch (err) {
    console.error("Failed to save sparkles:", err);
  }
}
```

- [ ] **Step 5: Call loadSparkles() during app init**

Find the `loadConfig()` call in `src/main.ts` (or wherever the app initializes). Add `loadSparkles()` alongside it. Check `src/main.ts` for the mount/init logic and add the import and call.

- [ ] **Step 6: Commit**

```bash
git add src/stores/config.ts src/main.ts
git commit -m "feat(sparkle): add sparkleStore, MODES entry, and config fields"
```

---

## Task 3: LLM client — sparkle system prompt and translate branching

**Files:**
- Modify: `src/services/llm-client.ts`

- [ ] **Step 1: Add import for sparkleStore**

In `src/services/llm-client.ts`, add `sparkleStore` to the import from `"../stores/config"` (line ~6):

```ts
import { loadConfig, saveConfig, getActiveModel, appConfig, personaStore, sparkleStore, refreshDictStatus } from "../stores/config";
```

- [ ] **Step 2: Add buildSparkleSystemPrompt function**

Add after `buildSystemPrompt()` (after line 235):

```ts
function buildSparkleSystemPrompt(): string {
  const enabled = sparkleStore.sparkles.find((s) => s.enabled);
  if (!enabled) {
    return "You are a helpful assistant. Output ONLY the result, nothing else.";
  }
  return (
    enabled.prompt +
    "\n\nIMPORTANT: Output ONLY the transformed result. Do not include any explanations, notes, meta-commentary, or original text. Output just the result."
  );
}
```

- [ ] **Step 3: Branch translate() by mode**

In the `translate()` function (starting at line 46), find the line:

```ts
const systemPrompt = buildSystemPrompt();
```

Replace with:

```ts
const mode = appConfig.active_mode || "translate";
const systemPrompt = mode === "sparkle"
  ? buildSparkleSystemPrompt()
  : buildSystemPrompt();
```

Then find the dictionary matching block (lines ~61-79, the `if (appConfig.user_dict_enabled)` block). Wrap it in a mode check so dictionary is skipped for sparkle:

Change:
```ts
if (appConfig.user_dict_enabled) {
```

To:
```ts
if (mode === "translate" && appConfig.user_dict_enabled) {
```

- [ ] **Step 4: Add mode parameter to optimizePrompt**

Change the `optimizePrompt` function signature from:

```ts
export async function optimizePrompt(rawPrompt: string): Promise<string> {
```

To:

```ts
export async function optimizePrompt(rawPrompt: string, mode: "translate" | "sparkle" = "translate"): Promise<string> {
```

Then replace the `content` in the system message (the long string at lines 152-161). Wrap it:

```ts
content: mode === "sparkle"
  ? "You organize and structure user-written prompts. Reorganize the prompt to be clear, well-structured, and unambiguous. Do not change the original intent or add new instructions. Output ONLY the reorganized prompt, nothing else."
  : "You optimize persona prompts for a translation tool. The user writes a vague style description in any language; you convert it into a concise instruction that assigns the LLM a professional role.\n" +
    "Output format: Start with \"You role as a [profession/role], using your professional vocabulary\", where [profession/role] is the most fitting English role name derived from the user's description. Under 20 words.\n" +
    'Examples:\n' +
    '- "像个影视专业人员" → "You role as a film and television professional, using your professional vocabulary."\n' +
    '- "正式一点" → "You role as a formal academic scholar, using your professional vocabulary."\n' +
    '- "口语化" → "You role as a casual native speaker, using your professional vocabulary."\n' +
    "- Output ONLY the optimized prompt, nothing else.",
```

- [ ] **Step 5: Commit**

```bash
git add src/services/llm-client.ts
git commit -m "feat(sparkle): add sparkle system prompt builder and mode branching in translate()"
```

---

## Task 4: i18n keys for sparkle

**Files:**
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh-CN.json`

- [ ] **Step 1: Add keys to en.json**

Add `"sparkle"` to the `"modes"` object (after `"translate": "Translate"`):

```json
"sparkle": "Sparkle"
```

Add these keys to the `"settings"` object (after `"optimizePrompt"`):

```json
"sparkleTitle": "Sparkle",
"noSparklesYet": "No sparkles yet.",
"addOneToSparkle": "Add one to get started.",
"addSparkles": "Add Sparkle",
"sparklePrompt": "System Prompt",
"organizePrompt": "AI Organize prompt",
"sparkleModel": "Sparkle Model"
```

Add these keys to the `"floating"` object:

```json
"selectSparkle": "Select Sparkle"
```

- [ ] **Step 2: Add corresponding keys to zh-CN.json**

Add `"sparkle"` to the `"modes"` object:

```json
"sparkle": "灵感"
```

Add to `"settings"`:

```json
"sparkleTitle": "灵感",
"noSparklesYet": "暂无灵感。",
"addOneToSparkle": "添加一个开始使用。",
"addSparkles": "添加灵感",
"sparklePrompt": "系统提示词",
"organizePrompt": "AI 条理化",
"sparkleModel": "灵感模型"
```

Add to `"floating"`:

```json
"selectSparkle": "选择灵感"
```

- [ ] **Step 3: Commit**

```bash
git add src/locales/en.json src/locales/zh-CN.json
git commit -m "feat(sparkle): add i18n keys for sparkle mode"
```

---

## Task 5: FloatingInput toolbar — sparkle selector

**Files:**
- Modify: `src/components/TranslateToolbar.vue`

- [ ] **Step 1: Add imports**

At the top of the `<script setup>`, add `sparkleStore` and `saveSparkles` to the imports from `"../stores/config"`:

```ts
import {
  appConfig,
  personaStore,
  sparkleStore,
  saveSparkles,
  getOrderedLanguages,
  dictStore,
  refreshDictStatus,
} from "../stores/config";
```

Add `Sparkles` to the lucide imports:

```ts
import {
  Languages,
  ChevronDown,
  UserCircle,
  BookText,
  Sparkles,
} from "@lucide/vue";
```

- [ ] **Step 2: Add sparkle state and functions**

Add after the existing persona state section (after `personaMenuRef`):

```ts
// ── Sparkle selector (sparkle mode) ──
const activeSparkleName = computed(() => {
  const s = sparkleStore.sparkles.find((s) => s.enabled);
  return s?.name || null;
});

const showSparkleDropdown = ref(false);
const sparkleDropdownRef = ref<HTMLDivElement | null>(null);
const sparkleBtnRef = ref<HTMLButtonElement | null>(null);
const sparkleMenuRef = ref<HTMLDivElement | null>(null);
const sparkleDropdownPos = ref({ top: 0, left: 0 });

function toggleSparkleDropdown() {
  if (!showSparkleDropdown.value && sparkleBtnRef.value) {
    const rect = sparkleBtnRef.value.getBoundingClientRect();
    const wrapLeft = sparkleDropdownRef.value?.getBoundingClientRect().left ?? rect.left;
    sparkleDropdownPos.value = { top: rect.bottom + 4, left: wrapLeft };
    showSparkleDropdown.value = true;
    nextTick(() => {
      if (sparkleMenuRef.value) {
        const menuH = sparkleMenuRef.value.offsetHeight;
        const spaceBelow = window.innerHeight - rect.bottom - 4;
        const spaceAbove = rect.top - 4;
        if (menuH > spaceBelow && menuH <= spaceAbove) {
          sparkleDropdownPos.value = { top: rect.top - menuH - 4, left: wrapLeft };
        }
      }
    });
  } else {
    showSparkleDropdown.value = false;
  }
}

function selectSparkle(index: number) {
  for (const s of sparkleStore.sparkles) s.enabled = false;
  sparkleStore.sparkles[index].enabled = true;
  showSparkleDropdown.value = false;
  saveSparkles();
  emit("result-stale");
}

const sparkleDropdownStyle = computed(() => capHeight(sparkleStore.sparkles.length));
```

- [ ] **Step 3: Update closeAllDropdowns and onDocumentClick**

In `closeAllDropdowns()` add:

```ts
showSparkleDropdown.value = false;
```

In `onDocumentClick`, add sparkle dropdown handling alongside persona and lang:

```ts
if (
  sparkleDropdownRef.value?.contains(target) ||
  sparkleMenuRef.value?.contains(target)
) {
  return;
}
showSparkleDropdown.value = false;
```

- [ ] **Step 4: Add sparkle template**

In the `<template>`, wrap the existing translate-mode content (language selector, persona toggle, dictionary toggle) in a `<template v-if="appConfig.active_mode !== 'sparkle'">` and add a new sparkle section. The sparkle section is a single dropdown button:

After the existing content, add:

```html
<!-- Sparkle selector (sparkle mode only) -->
<template v-if="appConfig.active_mode === 'sparkle'">
  <div class="sparkle-wrap" ref="sparkleDropdownRef">
    <button
      ref="sparkleBtnRef"
      @click="toggleSparkleDropdown"
      class="sparkle-btn"
      :class="{ active: showSparkleDropdown }"
      :title="t('floating.selectSparkle')"
    >
      <Sparkles :size="11" :stroke-width="1.8" />
      <span class="truncate max-w-[5em] min-w-0">{{ activeSparkleName }}</span>
      <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
        :style="{ transform: chevronTransform(showSparkleDropdown) }" />
    </button>

    <Teleport to="body">
      <Transition name="dropdown">
        <div
          v-if="showSparkleDropdown"
          ref="sparkleMenuRef"
          class="model-dropdown sparkle-dropdown"
          :style="{ top: sparkleDropdownPos.top + 'px', left: sparkleDropdownPos.left + 'px', ...sparkleDropdownStyle }"
        >
          <button
            v-for="(sparkle, si) in sparkleStore.sparkles"
            :key="si"
            @click="selectSparkle(si)"
            class="model-option"
            :class="{ selected: sparkle.enabled }"
          >
            <span class="truncate">{{ sparkle.name }}</span>
            <span v-if="sparkle.enabled" class="check-mark">&#10003;</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
```

Wrap the existing translate-mode elements (language selector, persona, dictionary) in `<template v-else>` so they only show in non-sparkle modes.

- [ ] **Step 5: Add sparkle button styles**

Add to `<style scoped>`:

```css
/* Sparkle selector */
.sparkle-wrap { display: inline-flex; flex-shrink: 0; }
.sparkle-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
  padding: 0 8px 0 7px;
  border-radius: 8px;
  font-size: 10px;
  font-weight: 550;
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  transition: all 0.15s ease;
}
.sparkle-btn:hover,
.sparkle-btn.active {
  color: var(--color-accent);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
}
```

- [ ] **Step 6: Commit**

```bash
git add src/components/TranslateToolbar.vue
git commit -m "feat(sparkle): add sparkle selector in FloatingInput toolbar"
```

---

## Task 6: Settings page — sparkle tab

**Files:**
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: Add imports**

Add `sparkleStore`, `saveSparkles as persistSparkles`, `loadSparkles` to the config imports:

```ts
import {
  appConfig,
  personaStore,
  sparkleStore,
  loadConfig,
  saveConfig as persistConfig,
  savePersonas as persistPersonas,
  saveSparkles as persistSparkles,
  getOrderedLanguages,
  loadProviderPresets,
  dictStore,
  refreshDictStatus,
  clearAllHistory,
  MODES,
} from "../stores/config";
```

Add `Sparkles` to the lucide imports:

```ts
import {
  ArrowLeft,
  Languages,
  UserCircle,
  Settings2,
  Plus,
  Trash2,
  Check,
  Eye,
  EyeOff,
  Link2,
  Loader2,
  RefreshCw,
  ChevronDown,
  Pencil,
  Cpu,
  CircleDot,
  X,
  BookText,
  GripVertical,
  RotateCcw,
  Wand2,
  Sun,
  Moon,
  SunMoon,
  Info,
  ToggleLeft,
  ToggleRight,
  Droplet,
  Database,
  Monitor,
  History,
  Sparkles,
} from "@lucide/vue";
```

- [ ] **Step 2: Add sparkle watcher and helper functions**

Add a watcher for sparkle auto-save (next to the existing persona watcher at ~419):

```ts
watch(
  () => JSON.stringify(sparkleStore.sparkles),
  () => { persistSparkles(); },
);
```

Add sparkle-specific validation and handler functions (next to the persona helpers at ~247):

```ts
function validateSparkle(s: { name: string; prompt: string }): string | null {
  const missing: string[] = [];
  if (!s.name.trim()) missing.push("Name");
  if (!s.prompt.trim()) missing.push("Prompt");
  return missing.length ? `Required: ${missing.join(", ")}` : null;
}

function toggleSparkle(index: number, e: MouseEvent) {
  const wasOn = sparkleStore.sparkles[index].enabled;
  for (const s of sparkleStore.sparkles) s.enabled = false;
  if (!wasOn) {
    sparkleStore.sparkles[index].enabled = true;
    burstParticles(e.currentTarget as HTMLElement);
  }
  persistSparkles();
}

async function handleOrganizePrompt(item: { prompt: string }, index: number) {
  if (!item.prompt.trim() || optimizingIndex.value !== null) return;
  promptUndoStack.set(index, item.prompt);
  optimizingIndex.value = index;
  try {
    item.prompt = await optimizePrompt(item.prompt, "sparkle");
  } catch (err) {
    console.error("Organize failed:", err);
    promptUndoStack.delete(index);
  } finally {
    optimizingIndex.value = null;
  }
}
```

- [ ] **Step 3: Add sparkle model selector helpers**

The sparkle model selector needs to bind to `sparkle_active_provider_index` / `sparkle_active_model_index`. Add:

```ts
function isSparkleModelActive(pIndex: number, mIndex: number): boolean {
  return pIndex === appConfig.sparkle_active_provider_index && mIndex === appConfig.sparkle_active_model_index;
}

const sparkleActiveLabel = computed(() => {
  const { providers } = appConfig;
  const pi = appConfig.sparkle_active_provider_index;
  const mi = appConfig.sparkle_active_model_index;
  if (pi >= providers.length) return "None";
  const p = providers[pi];
  if (!p || mi >= p.models.length) return "None";
  return p.models[mi].id;
});

function pickSparkleModel(e: FlatEntry) {
  appConfig.sparkle_active_provider_index = e.pIndex;
  appConfig.sparkle_active_model_index = e.mIndex;
  showModelSelector.value = false;
}
```

- [ ] **Step 4: Add sparkle tab to the Settings nav**

The existing `<nav class="tabs">` (around line 615) iterates `MODES` dynamically, so the sparkle tab appears automatically. No template change needed for the tab nav.

- [ ] **Step 5: Add sparkle tab content**

After the `</template>` that closes the translation tab (around line 1224), add the sparkle tab:

```html
<!-- ─── Sparkle tab ─── -->
<template v-if="activeTab === 'sparkle'">
  <!-- Model selector -->
  <div class="section-head">
    <span class="section-title"><Cpu :size="13" />{{ t('settings.sparkleModel') }}</span>
  </div>
  <div class="sel-wrap">
    <button
      ref="selBtnRef"
      class="sel-btn"
      :class="{ dead: allFlat.length === 0 }"
      @click="toggleSelMenu()"
    >
      <span class="sel-text">{{ allFlat.length === 0 ? t('settings.noModelsAvailable') : sparkleActiveLabel }}</span>
      <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showModelSelector }" />
    </button>

    <Teleport to="body">
      <Transition name="drop">
        <div v-if="showModelSelector && allFlat.length > 0" class="sel-menu" :style="{ top: selMenuPos.top + 'px', left: selMenuPos.left + 'px' }">
          <div class="sel-clip settings-scrollbar">
          <div class="sel-menu-inner">
            <button
              v-for="e in allFlat" :key="e.pIndex + '-' + e.mIndex"
              class="sel-opt"
              :class="{ hit: isSparkleModelActive(e.pIndex, e.mIndex) }"
              @click="pickSparkleModel(e)"
            >
              <div class="opt-info">
                <span class="opt-id">{{ e.id }}</span>
                <span class="opt-src">{{ e.providerName }}</span>
              </div>
              <Check
                v-if="isSparkleModelActive(e.pIndex, e.mIndex)"
                :size="13" :stroke-width="2.5"
              />
            </button>
          </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>

  <!-- Sparkle card list -->
  <EditableCardList
    class="mt"
    :items="sparkleStore.sparkles"
    :title="t('settings.sparkleTitle')"
    :icon="Sparkles"
    :empty-message="t('settings.noSparklesYet')"
    :empty-sub-message="t('settings.addOneToSparkle')"
    :validate="validateSparkle"
    :allow-remove="sparkleStore.sparkles.length > 1"
    :max-collapsed="5"
    @add="Object.assign($event, { name: '', prompt: '', enabled: false })"
    @confirm="() => persistSparkles()"
    @remove="() => persistSparkles()"
  >
    <template #collapsed="{ item, index }">
      <button class="about-auto-btn" :class="{ 'toggle-on': item.enabled }" @click.stop="toggleSparkle(index, $event)">
        <ToggleRight v-if="item.enabled" :size="15" :stroke-width="1.7" />
        <ToggleLeft v-else :size="15" :stroke-width="1.7" />
      </button>
      <span class="persona-name">{{ item.name }}</span>
    </template>

    <template #name-input="{ item }">
      <input v-model="item.name" :placeholder="t('settings.personaName')" class="fi name-fi" @click.stop />
    </template>

    <template #content="{ item, index }">
      <div class="persona-textarea-wrap">
        <textarea
          v-model="item.prompt"
          :placeholder="t('settings.sparklePrompt')"
          class="persona-textarea"
          rows="5"
          @click.stop
          @keydown="handleTextareaKeydown($event, item, index)"
        />
        <button
          v-if="item.prompt.trim()"
          class="persona-wand-btn"
          :class="{ active: optimizingIndex === index }"
          :title="t('settings.organizePrompt')"
          @click.stop="handleOrganizePrompt(item, index)"
        >
          <Loader2
            v-if="optimizingIndex === index"
            :size="12"
            :stroke-width="1.9"
            class="spin"
          />
          <Wand2 v-else :size="13" :stroke-width="1.6" />
        </button>
      </div>
    </template>
  </EditableCardList>
</template>
```

- [ ] **Step 6: Update EditableCardList to support maxCollapsed prop**

In `src/components/EditableCardList.vue`, add a `maxCollapsed` prop with default 3:

In the `withDefaults(defineProps<{...}>())` section, add:

```ts
const props = withDefaults(defineProps<{
  items: any[];
  title: string;
  icon: Component;
  emptyMessage?: string;
  emptySubMessage?: string;
  emptyIcon?: Component;
  validate?: (item: any) => string | null;
  allowRemove?: boolean;
  maxCollapsed?: number;
}>(), {
  allowRemove: true,
  maxCollapsed: 3,
});
```

Update the `.ecl-stack.compact` max-height calculation. The current hardcoded `168px` corresponds to 3 cards. Change the style to use a CSS custom property or inline style. In the template, on the `ecl-stack` div, add a dynamic style:

Change:
```html
<div ref="rootEl" class="ecl-stack" :class="{ compact: !adding && !isEditingAny }">
```

To:
```html
<div ref="rootEl" class="ecl-stack" :class="{ compact: !adding && !isEditingAny }" :style="(!adding && !isEditingAny) ? { maxHeight: (props.maxCollapsed * 56 + 4) + 'px' } : undefined">
```

And remove the hardcoded `max-height: 168px` from the `.ecl-stack.compact` CSS rule in `EditableCardList.vue` (line 328). Change:

```css
.ecl-stack.compact {
  max-height: 168px; overflow-y: auto; padding-right: 2px;
}
```

To:

```css
.ecl-stack.compact {
  overflow-y: auto; padding-right: 2px;
}
```

- [ ] **Step 7: Commit**

```bash
git add src/views/Settings.vue src/components/EditableCardList.vue
git commit -m "feat(sparkle): add sparkle settings tab with model selector and card list"
```

---

## Task 7: Wire up and test end-to-end

**Files:**
- Modify: `src/main.ts` (ensure `loadSparkles()` is called)

- [ ] **Step 1: Verify loadSparkles is called on app mount**

Read `src/main.ts` and confirm that `loadSparkles()` is called alongside `loadConfig()`. If not, add it.

- [ ] **Step 2: Run the app in dev mode**

Run: `cd c:/Users/Hong/Documents/Works_temp/prompit && npm run tauri dev`

Verify:
1. Settings shows a new "Sparkle" tab with the Sparkles icon
2. The tab contains a model selector and a sparkle card list with one pre-built "润色 (Polish)" sparkle
3. Switching to Sparkle mode in the floating input shows only a sparkle dropdown
4. Sending text in sparkle mode returns only the transformed result (no explanations)
5. Cannot delete the last sparkle (delete button hidden)

- [ ] **Step 3: Fix any issues found during testing**

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix(sparkle): address issues found during e2e testing"
```

---

## Task 8: Final cleanup commit

- [ ] **Step 1: Review all changes**

Run: `git diff master --stat`

Verify the file change list matches the spec.

- [ ] **Step 2: Final commit if needed**

```bash
git add -A
git commit -m "feat(sparkle): complete sparkle mode implementation"
```
