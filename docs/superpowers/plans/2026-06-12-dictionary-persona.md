# Dictionary Persona Column & Table Improvements — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add persona scoping to dictionary entries and improve the DictionaryEditor table with sorting, multi-select, and batch operations.

**Architecture:** Extend `DictEntry` with an optional `persona` field in both Rust and TypeScript. Update CSV import/export for backward-compatible 4-column format. Add persona-aware filtering in the translation pipeline. Enhance DictionaryEditor with a persona dropdown column, column sorting, row checkboxes, and batch action bar.

**Tech Stack:** Rust (Tauri commands, serde, csv crate), TypeScript, Vue 3 (Composition API), Tailwind CSS, vue-i18n.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `src-tauri/src/commands/dictionary.rs` | Modify | DictEntry struct, CSV import/export, dedup, tests |
| `src/stores/config.ts` | Modify | TS DictEntry interface |
| `src/services/llm-client.ts` | Modify | Persona-aware dictionary matching in `translate()` |
| `src/views/DictionaryEditor.vue` | Modify | Persona column, sorting, multi-select, batch ops |
| `src/locales/en.json` | Modify | New i18n keys |
| `src/locales/zh-CN.json` | Modify | New i18n keys |

---

### Task 1: Rust — DictEntry struct with persona field

**Files:**
- Modify: `src-tauri/src/commands/dictionary.rs`

- [ ] **Step 1: Update DictEntry struct**

Add `persona: Option<String>` with `#[serde(default)]` so existing data deserializes as `None`.

In `src-tauri/src/commands/dictionary.rs`, replace the struct definition (lines 8–12):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DictEntry {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub persona: Option<String>,
}
```

- [ ] **Step 2: Update existing roundtrip test**

Replace `test_dict_entry_serialize_roundtrip` (around line 232) to verify persona field:

```rust
#[test]
fn test_dict_entry_serialize_roundtrip() {
    let entry = DictEntry {
        source: "hello".into(),
        target: "你好".into(),
        persona: Some("Formal".into()),
    };
    let json = serde_json::to_string(&entry).unwrap();
    let back: DictEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(entry, back);
    assert_eq!(back.persona, Some("Formal".into()));
}
```

- [ ] **Step 3: Add backward-compat deserialization test**

Add a new test verifying old JSON (no persona field) deserializes with `None`:

```rust
#[test]
fn test_dict_entry_backward_compat() {
    let json = r#"{"source":"hello","target":"你好"}"#;
    let entry: DictEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.source, "hello");
    assert_eq!(entry.target, "你好");
    assert_eq!(entry.persona, None);
}
```

- [ ] **Step 4: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/dictionary.rs
git commit -m "feat(dictionary): add optional persona field to DictEntry"
```

---

### Task 2: Rust — CSV import with persona (backward compatible)

**Files:**
- Modify: `src-tauri/src/commands/dictionary.rs`

- [ ] **Step 1: Update CSV import logic**

In `import_dictionary_csv` (around line 142), replace the record parsing block:

```rust
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
    let persona = if record.len() >= 4 {
        let p = record[3].trim().to_string();
        if p.is_empty() { None } else { Some(p) }
    } else {
        None
    };
    parsed.push((lang, DictEntry { source, target, persona }));
}
```

- [ ] **Step 2: Update dedup key in "add" mode**

In the same function's "add" branch (around line 175), update the dedup check:

```rust
let exists = existing
    .iter()
    .any(|e| e.source == entry.source && e.target == entry.target && e.persona == entry.persona);
```

- [ ] **Step 3: Add CSV import test with persona**

Add after existing CSV tests:

```rust
#[test]
fn test_csv_import_with_persona() {
    let data = "lang,source,target,persona\nEnglish,hello,你好,Formal\nEnglish,world,世界,\n";
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data.as_bytes());
    let mut entries = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let persona = if record.len() >= 4 {
            let p = record[3].trim().to_string();
            if p.is_empty() { None } else { Some(p) }
        } else {
            None
        };
        entries.push(DictEntry {
            source: record[1].to_string(),
            target: record[2].to_string(),
            persona,
        });
    }
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].persona, Some("Formal".into()));
    assert_eq!(entries[1].persona, None);
}

#[test]
fn test_csv_import_legacy_3col() {
    let data = "lang,source,target\nEnglish,hello,你好\n";
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data.as_bytes());
    let mut entries = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let persona = if record.len() >= 4 {
            let p = record[3].trim().to_string();
            if p.is_empty() { None } else { Some(p) }
        } else {
            None
        };
        entries.push(DictEntry {
            source: record[1].to_string(),
            target: record[2].to_string(),
            persona,
        });
    }
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].persona, None);
}
```

- [ ] **Step 4: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/dictionary.rs
git commit -m "feat(dictionary): CSV import with optional persona column (backward compat)"
```

---

### Task 3: Rust — CSV export with persona column

**Files:**
- Modify: `src-tauri/src/commands/dictionary.rs`

- [ ] **Step 1: Update export header and row writing**

In `export_dictionary_csv` (around line 213), update:

```rust
wtr.write_record(["lang", "source", "target", "persona"])
    .map_err(|e| format!("write header: {e}"))?;
let mut langs: Vec<&String> = store.keys().collect();
langs.sort();
for lang in langs {
    for entry in &store[lang] {
        wtr.serialize((
            lang.as_str(),
            entry.source.as_str(),
            entry.target.as_str(),
            entry.persona.as_deref().unwrap_or(""),
        ))
        .map_err(|e| format!("write csv: {e}"))?;
    }
}
```

- [ ] **Step 2: Update existing CSV generate test**

Replace `test_csv_generate` to include persona:

```rust
#[test]
fn test_csv_generate() {
    let entries = vec![
        DictEntry {
            source: "hello".into(),
            target: "你好".into(),
            persona: Some("Formal".into()),
        },
        DictEntry {
            source: "world".into(),
            target: "世界".into(),
            persona: None,
        },
    ];
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    wtr.write_record(["lang", "source", "target", "persona"]).unwrap();
    wtr.serialize(("English", entries[0].source.as_str(), entries[0].target.as_str(), entries[0].persona.as_deref().unwrap_or(""))).unwrap();
    wtr.serialize(("English", entries[1].source.as_str(), entries[1].target.as_str(), entries[1].persona.as_deref().unwrap_or(""))).unwrap();
    wtr.flush().unwrap();
    let output = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
    assert!(output.contains("lang,source,target,persona"));
    assert!(output.contains("hello,你好,Formal"));
    assert!(output.contains("world,世界,"));
}
```

- [ ] **Step 3: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/dictionary.rs
git commit -m "feat(dictionary): CSV export with persona column"
```

---

### Task 4: TypeScript — DictEntry interface & i18n keys

**Files:**
- Modify: `src/stores/config.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh-CN.json`

- [ ] **Step 1: Update TS DictEntry interface**

In `src/stores/config.ts` line 47, replace:

```typescript
export interface DictEntry {
  source: string;
  target: string;
  persona?: string;  // undefined = All (no persona constraint)
}
```

- [ ] **Step 2: Add i18n keys to `src/locales/en.json`**

Add to the `dictionary` object:

```json
"persona": "Persona",
"personaAll": "All",
"personaNotFound": "Persona does not exist",
"batchDelete": "Delete selected",
"batchPersona": "Change persona"
```

- [ ] **Step 3: Add i18n keys to `src/locales/zh-CN.json`**

Add to the `dictionary` object:

```json
"persona": "角色",
"personaAll": "全部",
"personaNotFound": "该角色不存在",
"batchDelete": "删除选中",
"batchPersona": "更改角色"
```

- [ ] **Step 4: Commit**

```bash
git add src/stores/config.ts src/locales/en.json src/locales/zh-CN.json
git commit -m "feat(dictionary): add persona field to TS DictEntry, add i18n keys"
```

---

### Task 5: Translation matching with persona filter

**Files:**
- Modify: `src/services/llm-client.ts`

- [ ] **Step 1: Import personaStore**

At the top of `src/services/llm-client.ts`, add `personaStore` to the existing import from `../stores/config`:

```typescript
import {
  appConfig,
  personaStore,
} from "../stores/config";
```

(Keep any other existing imports from that module.)

- [ ] **Step 2: Update dictionary matching logic**

In the `translate` function, replace the existing dictionary matching block:

```typescript
if (appConfig.user_dict_enabled) {
    const allEntries = await loadDictionary(appConfig.target_lang);
    const matched = allEntries.filter((e) => text.includes(e.source));
```

With:

```typescript
if (appConfig.user_dict_enabled) {
    const allEntries = await loadDictionary(appConfig.target_lang);
    const activePersona = personaStore.personas.find((p) => p.enabled)?.name || null;
    const matched = allEntries.filter((e) => {
      if (!text.includes(e.source)) return false;
      if (!e.persona) return true;
      return e.persona === activePersona;
    });
```

- [ ] **Step 3: Commit**

```bash
git add src/services/llm-client.ts
git commit -m "feat(dictionary): persona-aware matching in translation pipeline"
```

---

### Task 6: DictionaryEditor — Persona column with dropdown

**Files:**
- Modify: `src/views/DictionaryEditor.vue`

This is the largest task. It adds the persona column, its dropdown, and invalid persona styling.

- [ ] **Step 1: Add personaStore import and persona options computed**

In the `<script setup>`, add to the existing config import:

```typescript
import {
  appConfig,
  personaStore,
  loadDictionary,
  saveDictionary,
  importDictionaryCsv,
  exportDictionaryCsv,
  clearAllDictionaries,
  getOrderedLanguages,
} from "../stores/config";
```

Add a computed for valid persona names and the dropdown options:

```typescript
/* ── Persona helpers ── */
const personaNames = computed(() => personaStore.personas.map(p => p.name));
const personaOptions = computed(() => [null, ...personaNames.value]);
function personaLabel(p: string | undefined): string {
  return p ?? t('dictionary.personaAll');
}
function isPersonaValid(p: string | undefined): boolean {
  if (!p) return true; // All is always valid
  return personaNames.value.includes(p);
}
```

- [ ] **Step 2: Update addEntry to not set persona**

Replace the existing `addEntry`:

```typescript
function addEntry() {
  entries.value.push({ source: "", target: "" });
  dirty.value = true;
}
```

No change needed — `persona` is undefined by default, which means "All".

- [ ] **Step 3: Update handleSave to preserve persona**

In `handleSave`, update the validation and save block. Replace lines 83–110:

```typescript
async function handleSave() {
  saveError.value = "";
  for (let i = 0; i < entries.value.length; i++) {
    const e = entries.value[i];
    const hasSource = e.source.trim() !== "";
    const hasTarget = e.target.trim() !== "";
    if (!hasSource && !hasTarget) {
      saveError.value = t('dictionary.rowIsEmpty', { n: i + 1 });
      return;
    }
    if (!hasSource) {
      saveError.value = t('dictionary.sourceRequired', { n: i + 1 });
      return;
    }
    if (!hasTarget) {
      saveError.value = t('dictionary.translationRequired', { n: i + 1 });
      return;
    }
  }
  const valid = entries.value
    .filter((e) => e.source.trim() !== "" && e.target.trim() !== "")
    .map((e) => {
      const entry: DictEntry = { source: e.source.trim(), target: e.target.trim() };
      if (e.persona) entry.persona = e.persona;
      return entry;
    });
  try {
    await saveDictionary(viewLang.value, valid);
    dirty.value = false;
  } catch (err) {
    saveError.value = "Failed to save dictionary.";
    console.error("Failed to save dictionary:", err);
  }
}
```

- [ ] **Step 4: Add persona dropdown state**

```typescript
/* ── Persona dropdown per row ── */
const openPersonaRow = ref<number | null>(null);
const personaDropdownRef = ref<HTMLDivElement | null>(null);
const personaBtnRefs = ref<Map<number, HTMLButtonElement>>(new Map());
const personaDropdownPos = ref({ top: 0, left: 0 });
```

- [ ] **Step 5: Add persona dropdown open/close functions**

```typescript
function togglePersonaDropdown(rowIdx: number, event: MouseEvent) {
  if (openPersonaRow.value === rowIdx) {
    openPersonaRow.value = null;
    return;
  }
  const btn = event.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  personaDropdownPos.value = { top: rect.bottom + 4, left: rect.left };
  openPersonaRow.value = rowIdx;
}

function selectPersona(rowIdx: number, persona: string | null) {
  if (persona === null) {
    delete entries.value[rowIdx].persona;
  } else {
    entries.value[rowIdx].persona = persona;
  }
  openPersonaRow.value = null;
  dirty.value = true;
}

function closePersonaDropdown(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".persona-dropdown") && !t.closest(".persona-btn")) {
    openPersonaRow.value = null;
  }
}
```

- [ ] **Step 6: Register/unregister click listener**

In `onMounted`, add:

```typescript
document.addEventListener("click", closePersonaDropdown);
```

In `onUnmounted`, add:

```typescript
document.removeEventListener("click", closePersonaDropdown);
```

- [ ] **Step 7: Update template — table header**

Replace the header row (lines 308–312):

```html
<div class="dict-row dict-header-row">
  <div class="dict-col col-source">{{ t('dictionary.source') }}</div>
  <div class="dict-col col-trans">{{ t('dictionary.translation') }}</div>
  <div class="dict-col col-persona">{{ t('dictionary.persona') }}</div>
  <div class="dict-col col-action"></div>
</div>
```

- [ ] **Step 8: Update template — data rows**

Replace the data row block (lines 315–340):

```html
<div v-for="(entry, i) in entries" :key="i" class="dict-row" :class="{ 'persona-invalid': entry.persona && !isPersonaValid(entry.persona) }">
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
  <div class="dict-col col-persona">
    <button
      class="persona-btn"
      :class="{ 'persona-missing': entry.persona && !isPersonaValid(entry.persona) }"
      :title="entry.persona && !isPersonaValid(entry.persona) ? t('dictionary.personaNotFound') : ''"
      @click="togglePersonaDropdown(i, $event)"
    >
      <span class="persona-label">{{ personaLabel(entry.persona) }}</span>
      <ChevronDown :size="10" :stroke-width="2" class="sel-arrow" :class="{ rot: openPersonaRow === i }" />
    </button>
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
```

- [ ] **Step 9: Add persona dropdown Teleport**

After the existing language dropdown Teleport, add:

```html
<!-- Persona dropdown per row -->
<Teleport to="body">
  <Transition name="drop">
    <div
      v-if="openPersonaRow !== null"
      class="sel-menu persona-dropdown"
      :style="{ top: personaDropdownPos.top + 'px', left: personaDropdownPos.left + 'px' }"
    >
      <div class="sel-clip settings-scrollbar">
        <div
          v-for="opt in personaOptions"
          :key="opt ?? '__all__'"
          class="sel-opt"
          :class="{ hit: (openPersonaRow !== null && entries[openPersonaRow]?.persona === opt) || (opt === null && openPersonaRow !== null && !entries[openPersonaRow]?.persona) }"
          @click="selectPersona(openPersonaRow!, opt)"
        >
          <span class="opt-label">{{ personaLabel(opt) }}</span>
          <Check
            v-if="openPersonaRow !== null && ((opt === null && !entries[openPersonaRow]?.persona) || entries[openPersonaRow]?.persona === opt)"
            :size="13" :stroke-width="2.5" class="lang-item-check"
          />
        </div>
      </div>
    </div>
  </Transition>
</Teleport>
```

- [ ] **Step 10: Add CSS for persona column**

Add to the `<style scoped>`:

```css
/* ── Persona column ── */
.col-persona {
  flex: 0 0 110px;
  border-right: 1px solid var(--color-border-hover);
  border-left: 1px solid var(--color-border-hover);
}
.persona-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: var(--color-text-secondary);
  background: none;
  border: 1px solid transparent;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  width: 100%;
}
.persona-btn:hover {
  border-color: var(--color-border-hover);
  background: var(--color-surface-hover);
}
.persona-label {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.persona-missing .persona-label {
  color: var(--color-text-muted);
  opacity: 0.5;
  text-decoration: line-through;
}
.persona-invalid .dict-input {
  opacity: 0.6;
}
```

- [ ] **Step 11: Commit**

```bash
git add src/views/DictionaryEditor.vue
git commit -m "feat(dictionary): add persona column with dropdown selector"
```

---

### Task 7: DictionaryEditor — Column sorting

**Files:**
- Modify: `src/views/DictionaryEditor.vue`

- [ ] **Step 1: Add sort state and computed**

Add after the persona helpers in `<script setup>`:

```typescript
/* ── Sorting ── */
const sortCol = ref<'source' | 'target' | 'persona'>('source');
const sortAsc = ref(true);

const sortedEntries = computed(() => {
  const arr = entries.value.map((e, i) => ({ entry: e, origIdx: i }));
  arr.sort((a, b) => {
    let cmp = 0;
    const col = sortCol.value;
    if (col === 'persona') {
      const pa = a.entry.persona ?? '';
      const pb = b.entry.persona ?? '';
      cmp = pa.localeCompare(pb);
    } else {
      cmp = (a.entry[col] ?? '').localeCompare(b.entry[col] ?? '');
    }
    return sortAsc.value ? cmp : -cmp;
  });
  return arr;
});

function toggleSort(col: 'source' | 'target' | 'persona') {
  if (sortCol.value === col) {
    sortAsc.value = !sortAsc.value;
  } else {
    sortCol.value = col;
    sortAsc.value = true;
  }
}
```

- [ ] **Step 2: Update template to use sortedEntries**

Replace the data row `v-for` loop. Change:

```html
<div v-for="(entry, i) in entries" :key="i"
```

To:

```html
<div v-for="{ entry, origIdx } in sortedEntries" :key="origIdx"
```

Update all references within the loop body from `i` to `origIdx`:

- `@click="togglePersonaDropdown(origIdx, $event)"`
- `@click="removeEntry(origIdx)"`
- `:class="{ rot: openPersonaRow === origIdx }"`
- All `entries[openPersonaRow]` references in the dropdown stay the same (openPersonaRow stores original index)

- [ ] **Step 3: Update header row with sort click handlers**

Replace header row:

```html
<div class="dict-row dict-header-row">
  <div class="dict-col col-source sortable" @click="toggleSort('source')">
    {{ t('dictionary.source') }}
    <component :is="sortCol === 'source' ? (sortAsc ? ChevronUp : ChevronDown) : undefined" v-if="sortCol === 'source'" :size="10" class="sort-arrow" />
  </div>
  <div class="dict-col col-trans sortable" @click="toggleSort('target')">
    {{ t('dictionary.translation') }}
    <component :is="sortCol === 'target' ? (sortAsc ? ChevronUp : ChevronDown) : undefined" v-if="sortCol === 'target'" :size="10" class="sort-arrow" />
  </div>
  <div class="dict-col col-persona sortable" @click="toggleSort('persona')">
    {{ t('dictionary.persona') }}
    <component :is="sortCol === 'persona' ? (sortAsc ? ChevronUp : ChevronDown) : undefined" v-if="sortCol === 'persona'" :size="10" class="sort-arrow" />
  </div>
  <div class="dict-col col-action"></div>
</div>
```

Add `ChevronUp` to the lucide import at the top of `<script setup>`:

```typescript
import { ArrowLeft, Download, Upload, Trash2, Plus, Save, ChevronDown, ChevronUp, Check, X } from "@lucide/vue";
```

- [ ] **Step 4: Add sort CSS**

```css
/* ── Sorting ── */
.sortable {
  cursor: pointer;
  user-select: none;
  gap: 4px;
}
.sortable:hover {
  color: var(--color-text-secondary);
}
.sort-arrow {
  color: var(--color-text-muted);
  flex-shrink: 0;
}
```

- [ ] **Step 5: Commit**

```bash
git add src/views/DictionaryEditor.vue
git commit -m "feat(dictionary): add column sorting to table"
```

---

### Task 8: DictionaryEditor — Multi-select & batch operations

**Files:**
- Modify: `src/views/DictionaryEditor.vue`

- [ ] **Step 1: Add multi-select state**

Add after sorting state:

```typescript
/* ── Multi-select ── */
const selectedSet = ref<Set<number>>(new Set());
const hasSelection = computed(() => selectedSet.value.size > 0);
const allSelected = computed(() =>
  entries.value.length > 0 && selectedSet.value.size === sortedEntries.value.length
);

function toggleSelect(origIdx: number) {
  if (selectedSet.value.has(origIdx)) {
    selectedSet.value.delete(origIdx);
  } else {
    selectedSet.value.add(origIdx);
  }
}

function toggleSelectAll() {
  if (allSelected.value) {
    selectedSet.value.clear();
  } else {
    selectedSet.value = new Set(sortedEntries.value.map(s => s.origIdx));
  }
}

function deleteSelected() {
  entries.value = entries.value.filter((_, i) => !selectedSet.value.has(i));
  selectedSet.value.clear();
  dirty.value = true;
  saveError.value = "";
}

/* ── Batch persona change ── */
const showBatchPersona = ref(false);
const batchPersonaPos = ref({ top: 0, left: 0 });

function openBatchPersona(e: MouseEvent) {
  const btn = e.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  batchPersonaPos.value = { top: rect.bottom + 4, left: rect.left };
  showBatchPersona.value = true;
}

function applyBatchPersona(persona: string | null) {
  for (const idx of selectedSet.value) {
    if (persona === null) {
      delete entries.value[idx].persona;
    } else {
      entries.value[idx].persona = persona;
    }
  }
  showBatchPersona.value = false;
  dirty.value = true;
}

function closeBatchDropdown(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".batch-persona-dropdown") && !t.closest(".batch-persona-btn")) {
    showBatchPersona.value = false;
  }
}
```

- [ ] **Step 2: Register/unregister batch click listener**

In `onMounted`, add:

```typescript
document.addEventListener("click", closeBatchDropdown);
```

In `onUnmounted`, add:

```typescript
document.removeEventListener("click", closeBatchDropdown);
```

- [ ] **Step 3: Update header row — add checkbox column**

Prepend checkbox column to the header row. The header row becomes:

```html
<div class="dict-row dict-header-row">
  <div class="dict-col col-check">
    <input type="checkbox" class="dict-checkbox" :checked="allSelected" @change="toggleSelectAll" />
  </div>
  <div class="dict-col col-source sortable" @click="toggleSort('source')">
    {{ t('dictionary.source') }}
    <component :is="sortCol === 'source' ? (sortAsc ? ChevronUp : ChevronDown) : undefined" v-if="sortCol === 'source'" :size="10" class="sort-arrow" />
  </div>
  <div class="dict-col col-trans sortable" @click="toggleSort('target')">
    {{ t('dictionary.translation') }}
    <component :is="sortCol === 'target' ? (sortAsc ? ChevronUp : ChevronDown) : undefined" v-if="sortCol === 'target'" :size="10" class="sort-arrow" />
  </div>
  <div class="dict-col col-persona sortable" @click="toggleSort('persona')">
    {{ t('dictionary.persona') }}
    <component :is="sortCol === 'persona' ? (sortAsc ? ChevronUp : ChevronDown) : undefined" v-if="sortCol === 'persona'" :size="10" class="sort-arrow" />
  </div>
  <div class="dict-col col-action"></div>
</div>
```

- [ ] **Step 4: Update data rows — add checkbox**

Prepend checkbox to each data row. The row becomes:

```html
<div v-for="{ entry, origIdx } in sortedEntries" :key="origIdx" class="dict-row" :class="{ 'persona-invalid': entry.persona && !isPersonaValid(entry.persona) }">
  <div class="dict-col col-check">
    <input type="checkbox" class="dict-checkbox" :checked="selectedSet.has(origIdx)" @change="toggleSelect(origIdx)" />
  </div>
  <div class="dict-col col-source">
    <input class="dict-input" v-model="entry.source" placeholder="..." @input="dirty = true" />
  </div>
  <div class="dict-col col-trans">
    <input class="dict-input" v-model="entry.target" placeholder="..." @input="dirty = true" />
  </div>
  <div class="dict-col col-persona">
    <button
      class="persona-btn"
      :class="{ 'persona-missing': entry.persona && !isPersonaValid(entry.persona) }"
      :title="entry.persona && !isPersonaValid(entry.persona) ? t('dictionary.personaNotFound') : ''"
      @click="togglePersonaDropdown(origIdx, $event)"
    >
      <span class="persona-label">{{ personaLabel(entry.persona) }}</span>
      <ChevronDown :size="10" :stroke-width="2" class="sel-arrow" :class="{ rot: openPersonaRow === origIdx }" />
    </button>
  </div>
  <div class="dict-col col-action">
    <button class="mini-btn warn" @click="removeEntry(origIdx)">
      <Trash2 :size="13" />
    </button>
  </div>
</div>
```

- [ ] **Step 5: Update lang row — toggle between Add Entry and batch actions**

Replace the `dict-lang-row` div (lines 293–302):

```html
<div class="dict-lang-row">
  <button ref="langBtnRef" class="sel-btn lang-sel-btn" @click="toggleLangMenu">
    <span class="sel-text">{{ getLangName(viewLang) }}</span>
    <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showLangMenu }" />
  </button>

  <!-- Batch actions (when selection active) -->
  <template v-if="hasSelection">
    <span class="batch-count">{{ selectedSet.size }}</span>
    <button class="pill-btn micro batch-persona-btn" @click="openBatchPersona">
      <UserCircle :size="12" />
      <span>{{ t('dictionary.batchPersona') }}</span>
    </button>
    <button class="pill-btn micro" @click="deleteSelected" style="color: var(--color-danger)">
      <Trash2 :size="12" />
      <span>{{ t('dictionary.batchDelete') }}</span>
    </button>
  </template>

  <!-- Add Entry (when no selection) -->
  <button v-else class="pill-btn add-pill" style="margin-left: auto" @click="addEntry">
    <Plus :size="12" :stroke-width="2" />
    <span>{{ t('dictionary.addEntry') }}</span>
  </button>
</div>
```

Add `UserCircle` to the lucide import:

```typescript
import { ArrowLeft, Download, Upload, Trash2, Plus, Save, ChevronDown, ChevronUp, Check, X, UserCircle } from "@lucide/vue";
```

- [ ] **Step 6: Add batch persona dropdown Teleport**

After the persona row dropdown Teleport:

```html
<!-- Batch persona dropdown -->
<Teleport to="body">
  <Transition name="drop">
    <div
      v-if="showBatchPersona"
      class="sel-menu batch-persona-dropdown"
      :style="{ top: batchPersonaPos.top + 'px', left: batchPersonaPos.left + 'px' }"
    >
      <div class="sel-clip settings-scrollbar">
        <div
          v-for="opt in personaOptions"
          :key="opt ?? '__all__'"
          class="sel-opt"
          @click="applyBatchPersona(opt)"
        >
          <span class="opt-label">{{ personaLabel(opt) }}</span>
        </div>
      </div>
    </div>
  </Transition>
</Teleport>
```

- [ ] **Step 7: Add multi-select CSS**

```css
/* ── Checkbox column ── */
.col-check {
  width: 32px;
  justify-content: center;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border-hover);
}
.dict-checkbox {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--color-accent);
}

/* ── Batch actions ── */
.batch-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-accent);
  background: var(--color-accent-bg);
  padding: 2px 8px;
  border-radius: 9px;
  margin-left: auto;
}
.batch-persona-btn:hover {
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
}
```

- [ ] **Step 8: Commit**

```bash
git add src/views/DictionaryEditor.vue
git commit -m "feat(dictionary): add multi-select, batch delete, batch persona change"
```

---

### Task 9: Manual verification

- [ ] **Step 1: Start dev server**

Run: `cd src-tauri && cargo tauri dev`

- [ ] **Step 2: Verify persona column**

1. Open Settings → Translation → User Dictionary
2. Confirm "Persona" column appears
3. Click persona cell → dropdown shows "All" + persona list
4. Select a persona → saves, displays correctly
5. Add new entry → persona defaults to "All"

- [ ] **Step 3: Verify invalid persona**

1. Add an entry with a persona
2. Go to Settings, delete that persona
3. Return to Dictionary → entry persona should be greyed/struck-through
4. Verify the entry does NOT match during translation

- [ ] **Step 4: Verify sorting**

1. Add several entries with different sources and personas
2. Click "Source" header → sorts alphabetically
3. Click again → reverses
4. Click "Persona" header → All (empty) first, then alphabetical

- [ ] **Step 5: Verify multi-select**

1. Check 2–3 rows
2. Confirm "Add Entry" replaced by batch action bar with count
3. Click batch persona → changes all selected
4. Click delete → removes all selected
5. Uncheck all → "Add Entry" returns

- [ ] **Step 6: Verify CSV import/export**

1. Export dictionary → CSV has 4 columns including persona
2. Import a legacy 3-column CSV → persona defaults to All
3. Import a 4-column CSV with persona → persona values preserved
