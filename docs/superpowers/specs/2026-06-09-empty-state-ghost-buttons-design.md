# Empty-State Ghost Buttons for Floating Input

**Date:** 2026-06-09
**Scope:** FloatingInput.vue, Settings.vue, i18n files

## Problem

When persona and user dictionary are empty, the persona and dictionary toggle buttons disappear entirely from the floating input toolbar. New users may never discover these features exist.

## Solution

Show lightweight "ghost" buttons when the stores are empty. Clicking a ghost button opens a hint modal that explains the feature and offers to navigate directly to the corresponding settings page.

## Design

### 1. Ghost Buttons

**Location:** Same toolbar position as the existing persona/dict toggle buttons, in both `growAbove` and `!growAbove` template branches.

**Visibility conditions:**
- Persona ghost: `personaStore.personas.length === 0`
- Dictionary ghost: `!dictStore.hasEntries`

These conditions are the exact inverse of the existing `v-if` on the real buttons. The ghost button and the real button are mutually exclusive.

**Visual treatment:**
- Same icons: `UserCircle` (persona), `BookText` (dictionary)
- Same height (28px) and padding as existing toolbar buttons
- Faint dashed border: `1px dashed var(--color-border)`
- Low opacity: `opacity: 0.4`
- No hover state: no background change, no opacity change, no cursor change
- No active/toggle state

**Click behavior:** Sets `emptyHintTarget` ref to `'persona'` or `'dict'`, which triggers the hint modal to appear.

### 2. Hint Modal

**Placement:** The modal is rendered once at the root level of the template (inside the outermost div), not duplicated per branch. The ghost buttons appear in both template branches, but they all control the same `emptyHintTarget` ref.

**Overlay:** Matches the onboarding exit-confirm pattern:
- `fixed inset-0`, centered with flexbox
- `background: rgba(0,0,0,0.4)`
- `backdrop-filter: blur(4px)`
- Vue `<Transition name="drop">` for enter/leave animation

**Card content:**
- Title line explaining the feature is empty
- Body line suggesting to add entries in Settings
- Two buttons:
  - **Cancel** — closes the modal (sets `emptyHintTarget` to `null`)
  - **Go to Settings** — navigates and closes

**Navigation on confirm:**
- Dictionary: `invoke("open_settings_window")` then `router.push('/settings/dictionary')`
- Persona: `invoke("open_settings_window")` then `router.push('/settings?tab=translation&scrollTo=persona')`

### 3. Scroll-to-Persona in Settings.vue

Add `id="persona-section"` to the persona `EditableCardList` wrapper element.

Extend the existing `onMounted` logic that handles `route.query.tab`:
```js
if (route.query.tab === "translation") {
  activeTab.value = "translation";
}
if (route.query.scrollTo === "persona") {
  nextTick(() => {
    document.getElementById("persona-section")?.scrollIntoView({ behavior: "smooth" });
  });
}
```

The dictionary page already has its own route (`/settings/dictionary`) so no scroll logic is needed there.

### 4. i18n Keys

New keys needed in both `en` and `zh-CN` locale files:

**Modal text:**
- `floating.emptyPersonaTitle` — "No translation persona configured"
- `floating.emptyPersonaBody` — "Add a persona in Settings to customize your translation style."
- `floating.emptyDictTitle` — "User dictionary is empty"
- `floating.emptyDictBody` — "Add terms in Settings for more accurate translations."

**Modal buttons:**
- `floating.goToSettings` — "Go to Settings"
- `floating.dismiss` (reuse if exists, otherwise add) — "Cancel"

### 5. Files Changed

| File | Change |
|------|--------|
| `src/views/FloatingInput.vue` | Ghost buttons (2 instances, one per template branch), hint modal, `emptyHintTarget` ref, navigation logic |
| `src/views/Settings.vue` | `id="persona-section"` on persona wrapper, scrollTo logic in onMounted |
| `src/i18n.ts` (or locale files) | New i18n keys for modal text |

### 6. What Does NOT Change

- Existing persona toggle/dropdown when personas exist — untouched
- Existing dictionary toggle when dict has entries — untouched
- `handleOpenSettings()` for the gear icon — untouched
- Dictionary route and DictionaryEditor — untouched
- Onboarding flow — untouched
