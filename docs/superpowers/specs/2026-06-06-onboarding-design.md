# Onboarding Flow Design

Date: 2026-06-06

## Overview

A multi-page first-time-use wizard that guides new users through adding an LLM provider and selecting a model before they can use the app. Triggers every time `appConfig.providers` is empty (including after a user deletes all providers).

## Architecture

**Approach:** Single Vue component (`Onboarding.vue`) with internal step management, following the existing pattern of `Settings.vue`.

**Route:** `/onboarding` — added to `src/router/index.ts`.

**Navigation guard (router beforeEach):**
- If `providers.length === 0` and target is not `/onboarding` → redirect to `/onboarding`
- If `providers.length > 0` and target is `/onboarding` → redirect to `/`

**Config loading:** `loadConfig()` is already awaited in `main.ts` before `app.mount()`, so by the time the router guard evaluates, `appConfig` has the real loaded data. No additional timing logic needed.

**Route theme:** Add `/onboarding` to `applyRouteTheme` in `main.ts` — use the same style as `/settings` (opaque background, scrollable).

## Steps

### Step 0 — Welcome

- Large "Hello" text
- Language selector dropdown (reuses `BUILTIN_LANGUAGES` + `appConfig.custom_languages`)
- Default: `appConfig.app_lang`
- Selection immediately applies: `appConfig.app_lang = selectedLang` (auto-save watcher persists it)
- Page text updates in real-time via vue-i18n

### Step 1 — Info

- Short text explaining the app requires an LLM model to function
- Explains the next steps will guide provider setup
- Pure informational, no interaction required

### Step 2 — Add Provider (Form)

- Preset selector dropdown (reuses `loadProviderPresets()`)
- Selecting a preset auto-fills: name, base_url, api_format
- User can manually override all fields
- Three inputs: Name, API Key (with show/hide toggle), Base URL
- Next button text changes to "Test & Continue"
- **Blocking conditions:** `providerForm.name && providerForm.api_key && providerForm.base_url`

### Step 3 — Select Models

- Auto-fetches model list from the provider's `/models` endpoint
- Displays as a checkbox list
- Select-all / deselect-all shortcut
- **Blocking condition:** `selectedModels.size > 0`
- If fetch fails: shows error + "Retry Fetch" button + "Back" button to step 2
- Finish button disabled until at least one model selected

### Step 4 — Done

- Congratulations text
- Platform-aware shortcut hint:
  - Windows/Linux: "Press Alt+Y to start"
  - macOS: "Press ⌥Y to start"
- Tip about adding more providers in Settings
- "Finish" button → writes models → navigates to `/`

## Step 3 → 4 Flow (Critical Path)

`confirmProviderAndAdvance()`:

1. Push `providerForm` to `appConfig.providers` (models = empty array)
2. `isConnecting = true` — test connectivity via GET `{base_url}{models_endpoint}`
3. **Fail** → show error, stay on step 2, user can edit and retry
4. **Success** → `isFetching = true`, fetch model list
5. **Fetch success** → populate `availableModels`, advance to step 4
6. **Fetch fail** → show error + retry button, stay on step 2

Reuses logic from `Settings.vue` `testConnection` / `fetchModels`. These will be extracted to `src/services/llm-client.ts` as shared functions (`testProviderConnection`, `fetchProviderModels`).

## Navigation

- Previous / Next buttons at bottom
- Step indicator: 5 dots, current highlighted, completed visually distinct
- Previous hidden on step 0
- Next disabled when `canProceed` is false
- Steps 3-4 buttons show loading state during async operations

## Transitions

- `<Transition>` with directional slide effect
- `direction === "forward"`: new page slides in from right
- `direction === "backward"`: new page slides in from left
- CSS transform + opacity, 300ms duration

## Data Persistence

All changes flow through the existing `appConfig` reactive + auto-save watcher:

- **Step 0:** `appConfig.app_lang` updated immediately
- **Step 2-3:** `providerForm` pushed to `appConfig.providers` on confirm
- **Step 4:** Selected models written to `appConfig.providers[0].models`, `active_provider_index = 0`, `active_model_index = 0`

## Shortcut Disabling During Onboarding

**Problem:** The global shortcut `Alt+Y` is registered in Rust (`shortcut.rs`). Even if the frontend ignores the event, the Rust handler still shows the window and navigates to `/`.

**Solution:**
- Add `onboarding_complete: Arc<AtomicBool>` to app state (default `false`)
- New Tauri command: `set_onboarding_complete` — sets the flag to `true`
- In `shortcut.rs` handler: check `onboarding_complete` before executing; if `false`, return early
- Frontend calls `invoke("set_onboarding_complete")` in step 4's finish action

## Platform-Aware Shortcut Display

Detect platform via `navigator.userAgent` or Tauri's `os.platform()`:
- Windows/Linux: display `Alt+Y`
- macOS: display `⌥Y`

## i18n

All onboarding text uses vue-i18n keys under `onboarding.*`. Minimum translations: `en` and `zh-CN`.

## Files Changed

| File | Change |
|------|--------|
| `src/views/Onboarding.vue` | **New** — main onboarding component |
| `src/router/index.ts` | Add `/onboarding` route + navigation guard |
| `src/services/llm-client.ts` | Extract `testProviderConnection` and `fetchProviderModels` as shared functions |
| `src/views/Settings.vue` | Refactor to use shared connection/model functions |
| `src-tauri/src/state.rs` | Add `onboarding_complete: Arc<AtomicBool>` |
| `src-tauri/src/lib.rs` | Register `set_onboarding_complete` command |
| `src-tauri/src/commands/` | New command module or add to existing for `set_onboarding_complete` |
| `src-tauri/src/shortcut.rs` | Check `onboarding_complete` flag before handling shortcut |
| `src/locales/en.json` | Add `onboarding.*` translation keys |
| `src/locales/zh-CN.json` | Add `onboarding.*` translation keys |
