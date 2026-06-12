# User Dictionary Persona Column & Table Improvements

**Date:** 2026-06-12
**Status:** Draft

## Overview

Extend the User Dictionary with a Persona column so entries can be scoped to specific personas. Add table-level UX improvements: column sorting, row multi-select, and batch operations.

## §1 Data Model

### Rust — `DictEntry`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DictEntry {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub persona: Option<String>,  // None = All (no persona constraint)
}
```

### TypeScript — `DictEntry`

```typescript
export interface DictEntry {
  source: string;
  target: string;
  persona?: string;  // undefined = All
}
```

### Migration

`#[serde(default)]` ensures existing entries without a `persona` field deserialize as `None`. No data migration needed. New entries default to `persona: undefined` (All).

## §2 CSV Import/Export

### Export

- Header: `lang,source,target,persona` (4 columns)
- `persona` = `None` → empty string in CSV
- `persona` = `Some("Formal")` → `"Formal"` in CSV

### Import

- 4-column rows: `lang, source, target, persona` — 4th column trimmed; empty → `None`, non-empty → `Some(value)`
- 3-column rows (legacy format): persona defaults to `None`. Fully backward compatible.
- Dedup key changes from `(lang, source, target)` to `(lang, source, target, persona)`, allowing same source→target pairs under different personas.

## §3 Translation Matching

**Current** (`llm-client.ts`):
```typescript
const matched = allEntries.filter((e) => text.includes(e.source));
```

**New**:
```typescript
const activePersona = personaStore.personas.find(p => p.enabled)?.name || null;

const matched = allEntries.filter((e) => {
  if (!text.includes(e.source)) return false;
  if (!e.persona) return true;              // All → always match
  return e.persona === activePersona;        // Must match active persona
});
```

Behavior matrix:

| Entry persona | Active persona | Match? |
|---|---|---|
| `undefined` (All) | Any | Yes |
| `"Formal"` | Formal enabled | Yes |
| `"Formal"` | Casual enabled | No |
| `"Formal"` | No persona enabled | No |
| `"Deleted persona"` | Any | No (never equals active) |

Entries with non-existent persona values naturally never match because `activePersona` can never equal a deleted persona name.

## §4 DictionaryEditor UI — Persona Column

### Layout

Table columns become: `☐ | Source | Translation | Persona | ✕`

### Persona Dropdown

- Per-row dropdown selector (not free text input)
- Options: `[All, ...personaStore.personas.map(p => p.name)]`
- "All" is a localized display label; stored as `undefined`
- New entries default to All

### Invalid Persona Display

When an entry's `persona` value does not exist in `personaStore.personas`:

- Persona cell text turns grey
- Tooltip: localized "Persona does not exist" message
- Dropdown still opens for the user to select a valid persona
- Grey status affects display and translation matching only; does not block saving
- Same behavior applies to imported CSV entries referencing non-existent personas

## §5 Column Sorting

### Interaction

- Click column header → sort ascending by that column
- Click same column again → reverse to descending
- Click different column → switch sort column, default ascending

### State

```typescript
const sortCol = ref<'source' | 'target' | 'persona'>('source');
const sortAsc = ref(true);
```

### Sort Rules

- Source / Translation: `localeCompare` string sort
- Persona: `undefined` (All) sorts first, then alphabetically by persona name
- Sorting is a `computed` view — does not mutate the `entries` array

### Visual Indicator

Active sort column header shows a small arrow icon (`↑` or `↓`) using existing `ChevronDown`/`ChevronUp` icons, color `var(--color-text-muted)`.

## §6 Multi-Select & Batch Operations

### Checkboxes

- New leftmost column (width ~32px) with checkbox per row
- Header row checkbox = select all / deselect all
- Selection state: `const selectedSet = ref<Set<number>>(new Set())` (indices in sorted view)

### Action Bar Toggle

- When `selectedSet.size > 0`: bottom `+ Add Entry` area is replaced by batch action bar
- When `selectedSet.size === 0`: `+ Add Entry` button restored

### Batch Actions (2 icon buttons)

- **Delete selected** (Trash icon) — deletes immediately, no confirmation (consistent with single-row delete)
- **Batch change Persona** (UserCircle icon) — opens dropdown; sets all selected entries to chosen persona (including "All")

### Cancel Selection

- Header checkbox deselects all
- Last checkbox unchecked restores `+ Add Entry`
- Sorting + multi-select: sorted view indices map back to original `entries` array for mutations

## Files Changed

| File | Change |
|---|---|
| `src-tauri/src/commands/dictionary.rs` | DictEntry struct, CSV import/export, dedup logic |
| `src/stores/config.ts` | TS DictEntry interface |
| `src/services/llm-client.ts` | Persona-aware matching in `translate()` |
| `src/views/DictionaryEditor.vue` | Persona column, sorting, multi-select, batch actions |
