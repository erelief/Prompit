# User Dictionary Improvements — Design Spec

Date: 2026-06-08

## Overview

Enhance the User Dictionary editor with whole-store import/export, confirmation flows, clear actions, and window draggability.

## Current State

- Dictionary is stored as `HashMap<String, Vec<DictEntry>>` keyed by target language name.
- Export/import operate on a single `target_lang` only.
- Import always appends (no dedup on source+target; only source is checked).
- No confirmation dialogs for import, no clear action, window is not draggable.
- DictionaryEditor is a route view inside the main Tauri window.

## Changes

### 1. Draggable Window

Mirror the Settings view pattern: add `@mousedown="handleDrag"` on `.dict-root` that calls `getCurrentWindow().startDragging()` unless the click target is an interactive element (`button, input, textarea, select, a`).

### 2. Export — All Languages, Timestamped Filename

- Export **all** languages into a single CSV.
- Filename: `Prompit_Translation_UD-YYMMDDHHmm.csv` (timestamp generated in TS, minute precision).
- CSV format — three columns:

```csv
lang,source,target
Japanese,hello,こんにちは
French,hello,bonjour
Simplified Chinese,world,世界
```

- Header row included (`lang,source,target`).

### 3. Import Confirmation Flow

When the user selects a CSV file:

1. **If all dictionaries are empty** (no entries for any language) → skip confirmation, import directly as "add" (add and overwrite are identical here).
2. **If any language has entries** → show a dialog with two choices:
   - **Add to existing** — merge new entries, dedup per entry.
   - **Overwrite per-language** — replace entries for languages present in the CSV; preserve languages not in the CSV.

### 4. Add Mode Deduplication

When importing with mode `"add"`:

- An entry is considered a duplicate only when **all three** fields match: `lang`, `source`, and `target`.
- Duplicate entries in the CSV are silently skipped.
- Non-duplicate entries are appended to the relevant language's list.

### 5. Overwrite Mode — Double Confirmation

When the user chooses "Overwrite":

- Show a warning: "This cannot be undone." (same copy pattern as EditableCardList's remove confirmation).
- Require a second explicit confirmation click before proceeding.
- Overwrite behavior: for each language present in the CSV, that language's entries are fully replaced. Languages not in the CSV remain untouched.

### 6. Empty Dictionary Shortcut

If the dictionary store has zero entries across all languages, import proceeds immediately without any confirmation dialog. Both add and overwrite produce the same result here.

### 7. Clear Actions

Two clear buttons in the footer area, both using the EditableCardList danger-confirm pattern (warning text + confirm button):

- **Clear current language** — removes all entries for the currently displayed target language only.
- **Clear all languages** — removes all entries for every language.

Both require double confirmation with "This cannot be undone." warning text.

### 8. Visual Consistency

Apply `/frontend-design` skill for final polish to ensure all new UI elements match the existing design system (`pill-btn`, `mini-btn`, `danger-bg`, `remove-warning-text` patterns).

## Backend API Changes

### Rust (`src-tauri/src/commands/dictionary.rs`)

Replace the current single-language import/export with whole-store versions:

```rust
#[tauri::command]
pub fn export_dictionary_csv(app: AppHandle, file_path: String) -> Result<(), String>
// Writes all languages to CSV with lang,source,target columns.

#[tauri::command]
pub fn import_dictionary_csv(
    app: AppHandle,
    file_path: String,
    mode: String,  // "add" | "overwrite"
) -> Result<ImportResult, String>
// mode="add": merge entries, dedup on (lang, source, target).
// mode="overwrite": per-language replace; languages not in CSV preserved.

#[derive(Serialize)]
pub struct ImportResult {
    pub total_entries: usize,    // total entries in store after import
    pub imported: usize,         // new entries actually added/replaced
    pub languages_affected: Vec<String>,
}
```

### TypeScript (`src/stores/config.ts`)

Update wrappers to match new Rust signatures:

```typescript
export async function importDictionaryCsv(
  filePath: string,
  mode: "add" | "overwrite"
): Promise<ImportResult>

export async function exportDictionaryCsv(
  filePath: string
): Promise<void>
```

Remove `lang` parameter from both functions.

### Command Registration (`src-tauri/src/lib.rs`)

Update `invoke_handler` registrations if signature names change.

### i18n Keys

New keys needed in both `en.json` and `zh-CN.json`:

```json
"dictionary": {
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
  "exportSuccess": "Dictionary exported.",
  "importSuccess": "Dictionary imported."
}
```

## Files to Modify

| File | Changes |
|---|---|
| `src/views/DictionaryEditor.vue` | Drag, import flow UI, clear buttons, export filename, confirmation dialogs |
| `src-tauri/src/commands/dictionary.rs` | Rewrite import/export for whole-store with mode parameter |
| `src/stores/config.ts` | Update TS wrappers for new Rust API |
| `src/locales/en.json` | New i18n keys |
| `src/locales/zh-CN.json` | New i18n keys |
| `src-tauri/src/lib.rs` | Update command registration if needed |
