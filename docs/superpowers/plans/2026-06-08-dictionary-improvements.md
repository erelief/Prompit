# Dictionary Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the User Dictionary editor with whole-store import/export, confirmation flows, clear actions, and window draggability.

**Architecture:** Rewrite the Rust backend import/export to operate on the full dictionary store (all languages) with a mode parameter. Frontend adds confirmation dialogs, clear buttons, drag support, and timestamped export filenames. All new UI follows the existing danger-confirm pattern from EditableCardList.

**Tech Stack:** Rust (Tauri commands, csv crate), Vue 3 (Composition API), vue-i18n, @tauri-apps/api

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `src-tauri/src/commands/dictionary.rs` | Modify | Rewrite import/export for whole-store; add `ImportResult` struct; add tests |
| `src/stores/config.ts` | Modify | Update TS wrappers to match new Rust signatures; add `ImportResult` type |
| `src/locales/en.json` | Modify | Add new i18n keys for dictionary section |
| `src/locales/zh-CN.json` | Modify | Add new i18n keys for dictionary section |
| `src/views/DictionaryEditor.vue` | Modify | Add drag, import confirmation, clear buttons, export timestamp |
| `docs/superpowers/specs/2026-06-08-dictionary-improvements-design.md` | Reference | Design spec |

---

### Task 1: Rewrite Rust `export_dictionary_csv` for whole-store export

**Files:**
- Modify: `src-tauri/src/commands/dictionary.rs:162-182`

The current export takes `target_lang` and exports one language. Replace it with a whole-store export that writes all languages in `lang,source,target` CSV format.

- [ ] **Step 1: Write the failing test**

Add this test inside `mod tests` (after the existing `test_csv_generate` test):

```rust
#[test]
fn test_csv_generate_multi_lang() {
    let mut store: HashMap<String, Vec<DictEntry>> = HashMap::new();
    store.insert(
        "Japanese".into(),
        vec![
            DictEntry { source: "hello".into(), target: "こんにちは".into() },
            DictEntry { source: "bye".into(), target: "さようなら".into() },
        ],
    );
    store.insert(
        "French".into(),
        vec![DictEntry { source: "hello".into(), target: "bonjour".into() }],
    );
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    wtr.write_record(["lang", "source", "target"]).unwrap();
    let mut langs: Vec<&String> = store.keys().collect();
    langs.sort();
    for lang in langs {
        for entry in &store[lang] {
            wtr.serialize((lang.as_str(), entry.source.as_str(), entry.target.as_str()))
                .unwrap();
        }
    }
    wtr.flush().unwrap();
    let output = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
    assert!(output.contains("lang,source,target"));
    assert!(output.contains("French,hello,bonjour"));
    assert!(output.contains("Japanese,hello,こんにちは"));
    assert!(output.contains("Japanese,bye,さようなら"));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cd src-tauri && cargo test test_csv_generate_multi_lang -- --nocapture`
Expected: PASS (test only exercises csv crate, no new command yet)

- [ ] **Step 3: Rewrite `export_dictionary_csv` command**

Replace the existing `export_dictionary_csv` function (lines 162–182) with:

```rust
#[tauri::command]
pub fn export_dictionary_csv(app: AppHandle, file_path: String) -> Result<(), String> {
    let store = load_dict_store(&app)?;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&file_path)
        .map_err(|e| format!("create csv: {e}"))?;
    wtr.write_record(["lang", "source", "target"])
        .map_err(|e| format!("write header: {e}"))?;
    let mut langs: Vec<&String> = store.keys().collect();
    langs.sort();
    for lang in langs {
        for entry in &store[lang] {
            wtr.serialize((lang.as_str(), entry.source.as_str(), entry.target.as_str()))
                .map_err(|e| format!("write csv: {e}"))?;
        }
    }
    wtr.flush().map_err(|e| format!("flush csv: {e}"))?;
    Ok(())
}
```

- [ ] **Step 4: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors (the TS wrapper still passes `targetLang` but that's fine for now — it's an unused extra field in the invoke, Tauri ignores extra args)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/dictionary.rs
git commit -m "refactor(dictionary): rewrite export for whole-store multi-lang CSV"
```

---

### Task 2: Rewrite Rust `import_dictionary_csv` with mode parameter

**Files:**
- Modify: `src-tauri/src/commands/dictionary.rs:119-160`

Add `ImportResult` struct and rewrite import to handle whole-store with `add`/`overwrite` mode.

- [ ] **Step 1: Add `ImportResult` struct**

Add this right after the `DictStore` type alias (after line 20):

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub total_entries: usize,
    pub imported: usize,
    pub languages_affected: Vec<String>,
}
```

- [ ] **Step 2: Write the failing test**

Add inside `mod tests`:

```rust
#[test]
fn test_csv_parse_multi_lang() {
    let data = "lang,source,target\nJapanese,hello,こんにちは\nFrench,hello,bonjour\n";
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data.as_bytes());
    let mut parsed: Vec<(String, DictEntry)> = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        if record.len() < 3 { continue; }
        let lang = record[0].trim().to_string();
        let source = record[1].trim().to_string();
        let target = record[2].trim().to_string();
        if lang.is_empty() || source.is_empty() || target.is_empty() { continue; }
        parsed.push((lang, DictEntry { source, target }));
    }
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].0, "Japanese");
    assert_eq!(parsed[1].0, "French");
}

#[test]
fn test_import_add_mode_dedup() {
    let mut store: DictStore = HashMap::new();
    store.insert(
        "Japanese".into(),
        vec![DictEntry { source: "hello".into(), target: "こんにちは".into() }],
    );
    let new_entries: Vec<(String, DictEntry)> = vec![
        ("Japanese".into(), DictEntry { source: "hello".into(), target: "こんにちは".into() }),
        ("Japanese".into(), DictEntry { source: "bye".into(), target: "さようなら".into() }),
        ("French".into(), DictEntry { source: "hello".into(), target: "bonjour".into() }),
    ];
    let mut langs_affected: Vec<String> = Vec::new();
    let mut imported = 0usize;
    for (lang, entry) in new_entries {
        let existing = store.entry(lang.clone()).or_default();
        let key = (entry.source.clone(), entry.target.clone());
        let exists: bool = existing.iter().any(|e| e.source == entry.source && e.target == entry.target);
        if !exists {
            existing.push(entry);
            imported += 1;
            if !langs_affected.contains(&lang) { langs_affected.push(lang.clone()); }
        }
    }
    let total: usize = store.values().map(|v| v.len()).sum();
    assert_eq!(imported, 2);
    assert_eq!(total, 3);
    assert!(langs_affected.contains(&"Japanese".to_string()));
    assert!(langs_affected.contains(&"French".to_string()));
}

#[test]
fn test_import_overwrite_mode() {
    let mut store: DictStore = HashMap::new();
    store.insert(
        "Japanese".into(),
        vec![DictEntry { source: "old".into(), target: "古い".into() }],
    );
    store.insert(
        "French".into(),
        vec![DictEntry { source: "keep".into(), target: "garder".into() }],
    );
    let new_entries: Vec<(String, DictEntry)> = vec![
        ("Japanese".into(), DictEntry { source: "new".into(), target: "新しい".into() }),
    ];
    for (lang, _) in &new_entries {
        store.remove(lang);
    }
    for (lang, entry) in new_entries {
        store.entry(lang).or_default().push(entry);
    }
    assert_eq!(store.get("Japanese").unwrap().len(), 1);
    assert_eq!(store.get("Japanese").unwrap()[0].source, "new");
    assert_eq!(store.get("French").unwrap().len(), 1);
    assert_eq!(store.get("French").unwrap()[0].source, "keep");
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd src-tauri && cargo test test_csv_parse_multi_lang test_import_add_mode_dedup test_import_overwrite_mode -- --nocapture`
Expected: all 3 PASS

- [ ] **Step 4: Rewrite `import_dictionary_csv` command**

Replace the existing `import_dictionary_csv` function (lines 119–160) with:

```rust
#[tauri::command]
pub fn import_dictionary_csv(
    app: AppHandle,
    file_path: String,
    mode: String,
) -> Result<ImportResult, String> {
    let raw = fs::read(&file_path).map_err(|e| format!("read file: {e}"))?;
    let data = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &raw[3..]
    } else {
        &raw
    };
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data);
    let mut parsed: Vec<(String, DictEntry)> = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("csv parse: {e}"))?;
        if record.len() < 3 {
            continue;
        }
        let lang = record[0].trim().to_string();
        let source = record[1].trim().to_string();
        let target = record[2].trim().to_string();
        if lang.is_empty() || source.is_empty() || target.is_empty() {
            continue;
        }
        parsed.push((lang, DictEntry { source, target }));
    }

    let mut store = load_dict_store(&app)?;
    let mut langs_affected: Vec<String> = Vec::new();
    let mut imported = 0usize;

    if mode == "overwrite" {
        let csv_langs: std::collections::HashSet<String> =
            parsed.iter().map(|(l, _)| l.clone()).collect();
        for lang in &csv_langs {
            store.remove(lang);
        }
        for (lang, entry) in parsed {
            store.entry(lang.clone()).or_default().push(entry);
            imported += 1;
            if !langs_affected.contains(&lang) {
                langs_affected.push(lang);
            }
        }
    } else {
        for (lang, entry) in parsed {
            let existing = store.entry(lang.clone()).or_default();
            let exists = existing
                .iter()
                .any(|e| e.source == entry.source && e.target == entry.target);
            if !exists {
                existing.push(entry);
                imported += 1;
                if !langs_affected.contains(&lang) {
                    langs_affected.push(lang);
                }
            }
        }
    }

    let total_entries: usize = store.values().map(|v| v.len()).sum();
    save_dict_store(&app, &store)?;
    langs_affected.sort();
    Ok(ImportResult {
        total_entries,
        imported,
        languages_affected: langs_affected,
    })
}
```

- [ ] **Step 5: Add `clear_all_dictionaries` Rust command**

Add this function right after `import_dictionary_csv` (after the function added in Step 4):

```rust
#[tauri::command]
pub fn clear_all_dictionaries(app: AppHandle) -> Result<(), String> {
    let store: DictStore = HashMap::new();
    save_dict_store(&app, &store)
}
```

- [ ] **Step 6: Register `clear_all_dictionaries` in `lib.rs`**

Add `commands::dictionary::clear_all_dictionaries,` to the `invoke_handler` array in `src-tauri/src/lib.rs` (after line 123, the existing `export_dictionary_csv` line):

```rust
commands::dictionary::clear_all_dictionaries,
```

- [ ] **Step 7: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 8: Run all dictionary tests**

Run: `cd src-tauri && cargo test dictionary -- --nocapture`
Expected: all tests PASS

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/commands/dictionary.rs src-tauri/src/lib.rs
git commit -m "feat(dictionary): rewrite import with add/overwrite mode and multi-lang support"
```

---

### Task 3: Update TypeScript wrappers in `config.ts`

**Files:**
- Modify: `src/stores/config.ts:331-349`

- [ ] **Step 1: Add `ImportResult` type and update wrappers**

Replace lines 331–349 (the `importDictionaryCsv` and `exportDictionaryCsv` functions) with:

```typescript
export interface ImportResult {
  total_entries: number;
  imported: number;
  languages_affected: string[];
}

export async function importDictionaryCsv(
  filePath: string,
  mode: "add" | "overwrite"
): Promise<ImportResult> {
  return await invoke<ImportResult>("import_dictionary_csv", {
    filePath,
    mode,
  });
}

export async function exportDictionaryCsv(
  filePath: string
): Promise<void> {
  await invoke("export_dictionary_csv", {
    filePath,
  });
}

export async function clearAllDictionaries(): Promise<void> {
  await invoke("clear_all_dictionaries");
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx vue-tsc --noEmit 2>&1 | head -20`
Expected: no errors related to dictionary (there may be pre-existing warnings)

- [ ] **Step 3: Commit**

```bash
git add src/stores/config.ts
git commit -m "refactor(config): update dictionary TS wrappers for multi-lang API"
```

---

### Task 4: Add i18n keys

**Files:**
- Modify: `src/locales/en.json:93-105`
- Modify: `src/locales/zh-CN.json:93-105`

- [ ] **Step 1: Update `en.json` dictionary section**

Replace the `"dictionary"` block (lines 93–105) with:

```json
"dictionary": {
  "userDictionary": "User Dictionary",
  "import": "Import",
  "export": "Export",
  "target": "Target",
  "addEntry": "Add Entry",
  "source": "Source",
  "translation": "Translation",
  "entries": "Entries",
  "rowIsEmpty": "Row {n} is empty — fill it or delete it.",
  "sourceRequired": "Row {n}: Translation requires a Source.",
  "translationRequired": "Row {n}: Source requires a Translation.",
  "importModeTitle": "Import Dictionary",
  "importModeHint": "How would you like to import this file?",
  "addToExisting": "Add to existing",
  "overwritePerLang": "Overwrite per-language",
  "overwriteWarning": "This cannot be undone.",
  "clearCurrent": "Clear current",
  "clearAll": "Clear all",
  "clearCurrentConfirm": "Clear all entries for {lang}?",
  "clearAllConfirm": "Clear all entries for all languages?",
  "imported": "Imported {n} entries across {langs} languages.",
  "noEntriesToExport": "No entries to export."
}
```

- [ ] **Step 2: Update `zh-CN.json` dictionary section**

Replace the `"dictionary"` block (lines 93–105) with:

```json
"dictionary": {
  "userDictionary": "用户词典",
  "import": "导入",
  "export": "导出",
  "target": "目标",
  "addEntry": "添加词条",
  "source": "原文",
  "translation": "译文",
  "entries": "词条数",
  "rowIsEmpty": "第 {n} 行为空 — 请填写或删除。",
  "sourceRequired": "第 {n} 行：译文需要原文。",
  "translationRequired": "第 {n} 行：原文需要译文。",
  "importModeTitle": "导入词典",
  "importModeHint": "你想如何导入这个文件？",
  "addToExisting": "添加到现有词典",
  "overwritePerLang": "按语言覆盖",
  "overwriteWarning": "此操作不可撤销。",
  "clearCurrent": "清空当前",
  "clearAll": "清空全部",
  "clearCurrentConfirm": "清空 {lang} 的所有词条？",
  "clearAllConfirm": "清空所有语言的所有词条？",
  "imported": "已导入 {n} 条，涉及 {langs} 种语言。",
  "noEntriesToExport": "没有可导出的词条。"
}
```

- [ ] **Step 3: Commit**

```bash
git add src/locales/en.json src/locales/zh-CN.json
git commit -m "feat(i18n): add dictionary improvement keys for en and zh-CN"
```

---

### Task 5: Update DictionaryEditor — drag, export filename, import flow, clear buttons

**Files:**
- Modify: `src/views/DictionaryEditor.vue`

This is the largest task. It modifies the script, template, and style sections of the component.

- [ ] **Step 1: Add imports and drag handler**

Add to the `<script setup>` imports. Replace the existing imports block (lines 1–15) with:

```typescript
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getLangName } from "../constants/languages";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  appConfig,
  loadDictionary,
  saveDictionary,
  importDictionaryCsv,
  exportDictionaryCsv,
  clearAllDictionaries,
} from "../stores/config";
import type { DictEntry } from "../stores/config";
import { ArrowLeft, Download, Upload, Trash2, Plus, Save } from "@lucide/vue";
```

Add the drag handler after `const dirty = ref(false);` (after line 22):

```typescript
/* ── Window drag ── */
async function handleDrag(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("textarea, button, input, select, a")) return;
  await getCurrentWindow().startDragging();
}
```

- [ ] **Step 2: Add import flow state and rewrite `handleImport`**

Add these refs after the drag handler:

```typescript
/* ── Import flow ── */
const showImportMode = ref(false);
const pendingImportPath = ref("");
const showOverwriteWarn = ref(false);
const importMessage = ref("");

function cancelImportMode() {
  showImportMode.value = false;
  showOverwriteWarn.value = false;
  pendingImportPath.value = "";
}

async function requestImport() {
  const filePath = await open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  pendingImportPath.value = filePath as string;
  // If dictionary is empty, skip confirmation
  const currentEntries = entries.value;
  const isEmpty = currentEntries.length === 0;
  if (isEmpty) {
    await executeImport("add");
  } else {
    showImportMode.value = true;
  }
}

async function chooseImportMode(mode: "add" | "overwrite") {
  if (mode === "overwrite") {
    showOverwriteWarn.value = true;
    return;
  }
  showImportMode.value = false;
  await executeImport("add");
}

async function confirmOverwrite() {
  showOverwriteWarn.value = false;
  showImportMode.value = false;
  await executeImport("overwrite");
}

async function executeImport(mode: "add" | "overwrite") {
  try {
    const result = await importDictionaryCsv(pendingImportPath.value, mode);
    entries.value = await loadDictionary(appConfig.target_lang);
    dirty.value = false;
    saveError.value = "";
    const langs = result.languages_affected.join(", ");
    importMessage.value = t('dictionary.imported', { n: result.imported, langs });
    setTimeout(() => { importMessage.value = ""; }, 4000);
  } catch (err) {
    console.error("Failed to import dictionary:", err);
  } finally {
    pendingImportPath.value = "";
  }
}
```

- [ ] **Step 3: Rewrite `handleExport` with timestamped filename**

Replace the existing `handleExport` function (lines 86–97) with:

```typescript
async function handleExport() {
  const now = new Date();
  const ts = [
    String(now.getFullYear()).slice(2),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
    String(now.getHours()).padStart(2, "0"),
    String(now.getMinutes()).padStart(2, "0"),
  ].join("");
  const filePath = await save({
    defaultPath: `Prompit_Translation_UD-${ts}.csv`,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  try {
    await exportDictionaryCsv(filePath);
  } catch (err) {
    console.error("Failed to export dictionary:", err);
  }
}
```

- [ ] **Step 4: Add clear flow state and handlers**

Add after the import flow section:

```typescript
/* ── Clear flow ── */
const pendingClear = ref<"current" | "all" | null>(null);

function requestClearCurrent() {
  pendingClear.value = "current";
}

function requestClearAll() {
  pendingClear.value = "all";
}

function cancelClear() {
  pendingClear.value = null;
}

async function confirmClear() {
  if (pendingClear.value === "current") {
    entries.value = [];
    await handleSave();
  } else if (pendingClear.value === "all") {
    await clearAllDictionaries();
    entries.value = [];
    dirty.value = false;
  }
  pendingClear.value = null;
}
```

- [ ] **Step 5: Rewrite the template**

Replace the entire `<template>` section with:

```html
<template>
  <div class="dict-root" @mousedown="handleDrag">
    <!-- Header -->
    <div class="dict-header">
      <button class="back-btn" @click="router.push('/settings?tab=translation')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('dictionary.userDictionary') }}</span>
      <button class="pill-btn micro" @click="requestImport">
        <Download :size="12" />
        <span>{{ t('dictionary.import') }}</span>
      </button>
      <button class="pill-btn micro" @click="handleExport">
        <Upload :size="12" />
        <span>{{ t('dictionary.export') }}</span>
      </button>
    </div>

    <!-- Language label + Add Entry -->
    <div class="dict-lang-row">
      <span class="dict-lang">{{ t('dictionary.target') }}: {{ getLangName(appConfig.target_lang) }}</span>
      <button class="pill-btn add-pill" @click="addEntry">
        <Plus :size="12" :stroke-width="2" />
        <span>{{ t('dictionary.addEntry') }}</span>
      </button>
    </div>

    <!-- Table -->
    <div class="dict-table-wrap">
      <div class="dict-table settings-scrollbar">
        <!-- Sticky header row -->
        <div class="dict-row dict-header-row">
          <div class="dict-col col-source">{{ t('dictionary.source') }}</div>
          <div class="dict-col col-trans">{{ t('dictionary.translation') }}</div>
          <div class="dict-col col-action"></div>
        </div>

        <!-- Data rows -->
        <div v-for="(entry, i) in entries" :key="i" class="dict-row">
          <div class="dict-col col-source">
            <input
              class="dict-input"
              v-model="entry.source"
              placeholder="..."
              @input="dirty = true"
            />
          </div>
          <div class="dict-col col-trans">
            <input
              class="dict-input"
              v-model="entry.target"
              placeholder="..."
              @input="dirty = true"
            />
          </div>
          <div class="dict-col col-action">
            <button
              class="mini-btn warn"
              @click="removeEntry(i)"
            >
              <Trash2 :size="13" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="dict-footer">
      <span class="footer-count">{{ t('dictionary.entries') }}: {{ entries.length }}</span>
      <span v-if="importMessage" class="footer-import-msg">{{ importMessage }}</span>
      <span v-if="saveError" class="footer-error">{{ saveError }}</span>

      <!-- Clear buttons -->
      <button class="pill-btn micro clear-btn" @click="requestClearCurrent" :disabled="entries.length === 0">
        <Trash2 :size="11" />
        <span>{{ t('dictionary.clearCurrent') }}</span>
      </button>
      <button class="pill-btn micro clear-btn" @click="requestClearAll">
        <Trash2 :size="11" />
        <span>{{ t('dictionary.clearAll') }}</span>
      </button>

      <button class="pill-btn save-btn" :disabled="!dirty" @click="handleSave">
        <Save :size="12" />
        <span>{{ t('common.save') }}</span>
      </button>
    </div>

    <!-- Import mode dialog -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="showImportMode" class="modal-overlay" @click.self="cancelImportMode">
          <div class="modal-card">
            <div class="modal-title">{{ t('dictionary.importModeTitle') }}</div>
            <div class="modal-hint">{{ t('dictionary.importModeHint') }}</div>
            <template v-if="!showOverwriteWarn">
              <div class="modal-actions">
                <button class="pill-btn modal-btn" @click="chooseImportMode('add')">
                  {{ t('dictionary.addToExisting') }}
                </button>
                <button class="pill-btn modal-btn warn-btn" @click="chooseImportMode('overwrite')">
                  {{ t('dictionary.overwritePerLang') }}
                </button>
              </div>
            </template>
            <template v-else>
              <div class="modal-warn-row">
                <span class="remove-warning-text">{{ t('dictionary.overwriteWarning') }}</span>
              </div>
              <div class="modal-actions">
                <button class="pill-btn modal-btn" @click="cancelImportMode">
                  {{ t('common.cancel') }}
                </button>
                <button class="pill-btn modal-btn danger-active" @click="confirmOverwrite">
                  {{ t('common.confirm') }}
                </button>
              </div>
            </template>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Clear confirm dialog -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="pendingClear" class="modal-overlay" @click.self="cancelClear">
          <div class="modal-card">
            <div class="modal-title">
              {{ pendingClear === 'current'
                ? t('dictionary.clearCurrentConfirm', { lang: getLangName(appConfig.target_lang) })
                : t('dictionary.clearAllConfirm') }}
            </div>
            <div class="modal-warn-row">
              <span class="remove-warning-text">{{ t('dictionary.overwriteWarning') }}</span>
            </div>
            <div class="modal-actions">
              <button class="pill-btn modal-btn" @click="cancelClear">
                {{ t('common.cancel') }}
              </button>
              <button class="pill-btn modal-btn danger-active" @click="confirmClear">
                {{ t('common.confirm') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
```

- [ ] **Step 6: Add modal styles**

Append these styles at the end of the `<style scoped>` section (before the closing `</style>` tag):

```css
/* ── Footer extras ── */
.footer-import-msg {
  flex: 1;
  color: var(--color-accent);
  font-weight: 500;
}
.clear-btn {
  color: var(--color-text-muted);
}
.clear-btn:hover:not(:disabled) {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.clear-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

/* ── Modal overlay ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999;
  backdrop-filter: blur(2px);
}
.modal-card {
  background: var(--color-bg);
  border: 1px solid var(--color-surface);
  border-radius: 11px;
  padding: 20px 24px;
  min-width: 280px;
  max-width: 360px;
  box-shadow: 0 8px 32px rgba(0,0,0,.25);
}
.modal-title {
  font-size: 13px;
  font-weight: 700;
  margin-bottom: 8px;
  color: var(--color-text);
}
.modal-hint {
  font-size: 11.5px;
  color: var(--color-text-muted);
  margin-bottom: 16px;
}
.modal-warn-row {
  margin-bottom: 12px;
}
.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.modal-btn {
  padding: 5px 14px;
  font-size: 11px;
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border-radius: 7px;
}
.modal-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.warn-btn {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.warn-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.danger-active {
  color: var(--color-danger);
  background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
@keyframes danger-pulse {
  from { opacity: .75; }
  to { opacity: 1; }
}

/* ── Modal transition ── */
.drop-enter-active,
.drop-leave-active {
  transition: opacity .15s ease;
}
.drop-enter-from,
.drop-leave-to {
  opacity: 0;
}
```

- [ ] **Step 7: Verify it compiles**

Run: `npx vue-tsc --noEmit 2>&1 | head -20`
Expected: no errors

- [ ] **Step 8: Commit**

```bash
git add src/views/DictionaryEditor.vue
git commit -m "feat(dictionary): add drag, import flow, clear buttons, export timestamp"
```

---

### Task 6: Remove unused `handleImport` function

The old `handleImport` was replaced by `requestImport` in Task 5. Verify the codebase has no references to the old function.

- [ ] **Step 1: Verify no stale references**

Search DictionaryEditor.vue for `handleImport`. It should not exist — replaced by `requestImport`, `chooseImportMode`, `confirmOverwrite`, `executeImport`.

Run: `grep -n "handleImport" src/views/DictionaryEditor.vue`
Expected: no output

- [ ] **Step 2: Build and smoke test**

Run: `npm run build`
Expected: builds successfully

- [ ] **Step 3: Commit if any cleanup was needed**

```bash
git add -A
git commit -m "chore(dictionary): clean up stale references"
```

(If no changes needed, skip this commit.)

---

### Task 7: Visual polish with `/frontend-design`

**Files:**
- Modify: `src/views/DictionaryEditor.vue` (style adjustments)

This task invokes the `/frontend-design` skill to review and polish the visual design of the new elements against the existing design system.

- [ ] **Step 1: Invoke `/frontend-design` skill**

Run the frontend-design skill to review:
- Modal dialog styling consistency with existing app
- Clear button placement in footer
- Import mode dialog visual hierarchy
- Danger-confirm animation consistency with EditableCardList

Apply any adjustments the skill recommends.

- [ ] **Step 2: Commit**

```bash
git add src/views/DictionaryEditor.vue
git commit -m "style(dictionary): polish UI elements for design consistency"
```

---

### Task 8: Final integration test

- [ ] **Step 1: Run full Rust test suite**

Run: `cd src-tauri && cargo test`
Expected: all tests pass

- [ ] **Step 2: Build the app**

Run: `npm run build`
Expected: builds successfully

- [ ] **Step 3: Manual smoke test checklist**

Run the app and verify:
1. Dictionary window is draggable by clicking the header area
2. Export produces a file named `Prompit_Translation_UD-YYMMDDHHmm.csv` with `lang,source,target` columns
3. Import with empty dictionary → imports directly, no confirmation
4. Import with existing entries → shows add/overwrite choice
5. Choosing overwrite → shows "This cannot be undone" warning + confirm
6. Add mode → duplicates (same lang+source+target) are skipped
7. "Clear current" button → shows confirm dialog, clears only current language
8. "Clear all" button → shows confirm dialog, clears all languages
9. All new UI elements match existing visual style

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat(dictionary): complete dictionary improvements"
```
