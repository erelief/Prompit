# Sparkle (灵感) Mode — Design Spec

**Date:** 2026-06-12
**Branch:** `feat/sparkle-mode`

## Summary

Add a new "Sparkle" mode to Prompit. Sparkle acts as an enhanced persona system: each sparkle is a user-defined system prompt that transforms user input in a fixed way. The mode ships with a built-in "Polish" sparkle. The floating toolbar in sparkle mode shows only a sparkle selector. At least one sparkle must exist at all times.

## 1. Data Model

### 1.1 SparkleEntry (Rust)

```rust
pub struct SparkleEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}
```

Identical shape to `PersonaEntry`. Stored in an encrypted `sparkles.json` file using the same AES-256-GCM scheme as personas.

### 1.2 AppConfig additions

```ts
sparkle_active_provider_index: number;  // default 0
sparkle_active_model_index: number;     // default 0
```

No other config fields needed. Sparkle mode shares `providers` with all other modes.

### 1.3 Frontend store

```ts
export const sparkleStore = reactive<{ sparkles: SparkleEntry[] }>({ sparkles: [] });
```

Symmetric to `personaStore`. Load/save via Tauri commands `read_sparkles` / `save_sparkles`.

### 1.4 Built-in Polish sparkle

On first load (when `sparkles.json` does not exist or is empty), the frontend `loadSparkles()` function auto-creates the default sparkle in the store and persists it:

- **Name:** `润色 (Polish)`
- **Prompt:**
  > Detect the language of the user's input. Adopt the role of a native speaker of that language. Rewrite the user's input as a more idiomatic, accurate, and natural expression in the same language, preserving the original meaning and intent.
- **Enabled:** true

This handles both fresh installs and upgrades from older versions. The Rust `read_sparkles` command returns an empty array when the file doesn't exist; the frontend detects the empty array and seeds the default.

## 2. Mode Registration

Add to `MODES` array in `config.ts`:

```ts
{
  id: "sparkle",
  icon: Sparkles,          // from @lucide/vue
  labelKey: "modes.sparkle",
  settingTabKey: "sparkle",
}
```

`getActiveModel()` already resolves mode-specific provider/model indices dynamically (`${mode}_active_provider_index`), so sparkle mode works automatically once the two new config fields exist.

## 3. LLM Call

### 3.1 buildSparkleSystemPrompt()

New function in `llm-client.ts`:

```
1. Find the enabled sparkle from sparkleStore.
2. Use its prompt as the base system message.
3. Append a hidden suffix (user-invisible, high-weight):
   "\n\nIMPORTANT: Output ONLY the transformed result. Do not include any explanations, notes, meta-commentary, or original text. Output just the result."
4. Return the combined string.
```

### 3.2 translate() branching

In `translate()`, branch on `appConfig.active_mode`:

- `"translate"` — existing `buildSystemPrompt()` + dictionary logic
- `"sparkle"` — `buildSparkleSystemPrompt()`, no dictionary, no language targeting

### 3.3 Prompt optimization (条理化)

`optimizePrompt()` gains a `mode` parameter:

- `"translate"` — current behavior (LLM optimizes translation prompt)
- `"sparkle"` — system instruction tells LLM: "Reorganize this prompt to be clear and structured. Do not change the intent."

## 4. FloatingInput Toolbar

`TranslateToolbar.vue` conditionally renders based on `appConfig.active_mode`:

**Sparkle mode renders only:**
- A sparkle selector dropdown (similar to the existing model selector pattern)
- Shows sparkle names only, with a checkmark on the active one
- Clicking a sparkle name enables it (disables all others)
- No language selector, no persona toggle, no dictionary toggle
- Since at least one sparkle always exists, no empty ghost state needed

The sparkle selector is visually consistent with the persona dropdown but simpler (always-on, no toggle step).

## 5. Settings Page

### 5.1 Sparkle tab

A new tab appears in Settings with the Sparkles icon. Contains only:

1. **Model selector** — identical to translate tab's model selector, but binds to `sparkle_active_provider_index` / `sparkle_active_model_index`
2. **Sparkle card list** — uses `EditableCardList` component

### 5.2 EditableCardList differences from persona

| Aspect | Persona | Sparkle |
|---|---|---|
| Collapsed slots visible | 3 (max-height ~168px) | 5 (max-height ~260px) |
| Edit textarea rows | 3 | 5 |
| Prompt optimization | Translation-style | 条理化 (organize/structure) |
| allowRemove | Always allowed | Only when `sparkles.length > 1` |

The sparkle card collapsed view shows:
- Toggle button (enabled/disabled)
- Sparkle name

The sparkle card expanded view shows:
- Name input
- Prompt textarea (taller)
- Wand button (条理化 optimization)
- Confirm/Cancel

## 6. Rust Backend

### 6.1 New file: `src-tauri/src/commands/sparkle.rs`

Copy `persona.rs` structure with these changes:
- File stored as `sparkles.json`
- Functions: `read_sparkles`, `save_sparkles`
- Same encryption (derive_key, AES-256-GCM)
- Same `SparkleEntry` struct as described above

### 6.2 `lib.rs` registration

Register `read_sparkles` and `save_sparkles` commands.

## 7. i18n Keys

New keys needed:

```
modes.sparkle = "灵感" / "Sparkle"
settings.sparkleTitle = "灵感" / "Sparkle"
settings.noSparklesYet = "暂无灵感" / "No sparkles yet"
settings.addOneToSparkle = "添加一个开始使用" / "Add one to get started"
settings.sparklePrompt = "系统提示词" / "System Prompt"
settings.organizePrompt = "条理化" / "Organize"
floating.selectSparkle = "选择灵感" / "Select Sparkle"
```

## 8. File Change Summary

| File | Change |
|---|---|
| `src/stores/config.ts` | Add `sparkle_active_provider_index`, `sparkle_active_model_index` to AppConfig; add `sparkleStore`, `loadSparkles`, `saveSparkles`; add sparkle to `MODES` |
| `src/services/llm-client.ts` | Add `buildSparkleSystemPrompt()`; branch `translate()` by mode; add `mode` param to `optimizePrompt()` |
| `src/components/TranslateToolbar.vue` | Add sparkle mode rendering (sparkle dropdown selector) |
| `src/views/Settings.vue` | Add sparkle tab (model selector + sparkle card list with EditableCardList) |
| `src-tauri/src/commands/sparkle.rs` | New file — encrypted sparkle storage (copy of persona.rs adapted) |
| `src-tauri/src/lib.rs` | Register `read_sparkles`, `save_sparkles` |
| `src/i18n.ts` or locale JSON | Add sparkle-related i18n keys |

## 9. Constraints

- At least one sparkle must always exist (enforced by `allowRemove` condition and Rust-side validation)
- The hidden system prompt suffix is appended at call time, never stored or shown to the user
- Sparkle mode returns only the transformed result — no explanations, no original text
- History entries for sparkle mode use `mode: "sparkle"` (existing per-mode filtering works automatically)
- Onboarding flow is not affected — sparkle mode uses the same providers configured during onboarding
