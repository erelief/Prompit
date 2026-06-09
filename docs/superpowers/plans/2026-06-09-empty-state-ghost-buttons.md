# Empty-State Ghost Buttons Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show discoverable ghost buttons when persona/dictionary are empty, with a hint modal that navigates to the correct settings page.

**Architecture:** Add ghost button variants (dashed border, low opacity, no hover) alongside the existing toggle buttons in FloatingInput.vue's toolbar. A single shared modal (driven by `emptyHintTarget` ref) appears on click, matching the onboarding exit-confirm overlay pattern. Navigation uses existing router paths with a new `scrollTo` query param for persona.

**Tech Stack:** Vue 3 (Composition API), vue-i18n, vue-router, Tauri v2 invoke, Lucide icons

**Spec:** `docs/superpowers/specs/2026-06-09-empty-state-ghost-buttons-design.md`

---

### Task 1: Add i18n keys

**Files:**
- Modify: `src/locales/en.json` (inside `"floating"` object)
- Modify: `src/locales/zh-CN.json` (inside `"floating"` object)

- [ ] **Step 1: Add keys to en.json**

Add these keys after `"dictionary": "Dictionary"` inside the `"floating"` section of `src/locales/en.json`:

```json
"emptyPersonaTitle": "No translation persona configured",
"emptyPersonaBody": "Add a persona in Settings to customize your translation style.",
"emptyDictTitle": "User dictionary is empty",
"emptyDictBody": "Add terms in Settings for more accurate translations.",
"goToSettings": "Go to Settings"
```

The `"floating"` section should end up as:

```json
"floating": {
  "pressEnterToPaste": "Press Enter to paste result...",
  "typeToSend": "Type to send...",
  "sending": "Sending...",
  "pasteIntoActiveField": "Paste into active field (Enter)",
  "send": "Send (Enter)",
  "targetLanguage": "Target Language",
  "noPersonasAvailable": "No personas available",
  "enablePersona": "Enable persona",
  "disablePersona": "Disable persona",
  "enableDict": "Enable user dictionary",
  "disableDict": "Disable user dictionary",
  "dictionary": "Dictionary",
  "emptyPersonaTitle": "No translation persona configured",
  "emptyPersonaBody": "Add a persona in Settings to customize your translation style.",
  "emptyDictTitle": "User dictionary is empty",
  "emptyDictBody": "Add terms in Settings for more accurate translations.",
  "goToSettings": "Go to Settings"
}
```

- [ ] **Step 2: Add keys to zh-CN.json**

Add these keys after `"dictionary": "词典"` inside the `"floating"` section of `src/locales/zh-CN.json`:

```json
"emptyPersonaTitle": "未配置翻译角色",
"emptyPersonaBody": "在设置中添加角色以自定义翻译风格。",
"emptyDictTitle": "用户词典为空",
"emptyDictBody": "在设置中添加词条以获得更准确的翻译。",
"goToSettings": "前往设置"
```

- [ ] **Step 3: Commit**

```bash
git add src/locales/en.json src/locales/zh-CN.json
git commit -m "feat(i18n): add empty-state hint modal keys for persona and dictionary"
```

---

### Task 2: Add script logic to FloatingInput.vue

**Files:**
- Modify: `src/views/FloatingInput.vue` (script section)

This task adds the reactive state and handler functions. No template/CSS changes yet.

- [ ] **Step 1: Add emptyHintTarget ref**

In `src/views/FloatingInput.vue`, after the `showPersonaDropdown` ref block (around line 116), add:

```typescript
// ── Empty-state hint modal ──
const emptyHintTarget = ref<'persona' | 'dict' | null>(null);
```

- [ ] **Step 2: Add navigation function**

After the `handleOpenSettings` function (around line 316), add:

```typescript
async function handleEmptyHintGo() {
  const target = emptyHintTarget.value;
  emptyHintTarget.value = null;
  if (target === 'dict') {
    await invoke("open_settings_window");
    router.push('/settings/dictionary');
  } else if (target === 'persona') {
    await invoke("open_settings_window");
    router.push('/settings?tab=translation&scrollTo=persona');
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/views/FloatingInput.vue
git commit -m "feat(floating-input): add emptyHintTarget state and navigation handler"
```

---

### Task 3: Add ghost buttons to FloatingInput.vue template

**Files:**
- Modify: `src/views/FloatingInput.vue` (template section)

The template has two branches: `v-if="growAbove"` (line ~395) and `v-else` (line ~602). Each branch has a toolbar row with persona and dict buttons. We add ghost buttons that appear when the stores are empty — they are mutually exclusive with the existing real buttons.

- [ ] **Step 1: Add ghost buttons in the growAbove toolbar**

In the `v-if="growAbove"` toolbar section, after the persona `<div>` block (which ends with `</div>` around line 571) and before the dictionary toggle, insert the persona ghost button. Then after the dict toggle block, add the dict ghost button.

**Persona ghost** — insert immediately after the persona `</div>` wrapper (the one with `v-if="personaStore.personas.length > 0"`), before the `<!-- Dictionary toggle -->` comment:

```vue
	          <!-- Persona ghost (empty state) -->
	          <button
	            v-if="personaStore.personas.length === 0"
	            class="ghost-btn"
	            @click="emptyHintTarget = 'persona'"
	            :title="t('floating.noPersonasAvailable')"
	          >
	            <UserCircle :size="11" :stroke-width="1.8" />
	          </button>
```

**Dictionary ghost** — insert immediately after the `</button>` of the existing dict toggle (the one with `v-if="dictStore.hasEntries"`), before `<div class="flex-1"></div>`:

```vue
	          <!-- Dictionary ghost (empty state) -->
	          <button
	            v-if="!dictStore.hasEntries"
	            class="ghost-btn"
	            @click="emptyHintTarget = 'dict'"
	            :title="t('floating.dictionary')"
	          >
	            <BookText :size="11" :stroke-width="1.8" />
	          </button>
```

- [ ] **Step 2: Add the same ghost buttons in the !growAbove toolbar**

The `v-else` branch (starting around line 602) has the same toolbar structure. Repeat the same two insertions at the corresponding positions:

- Persona ghost after the persona `</div>` wrapper
- Dict ghost after the existing dict toggle `</button>`

The code is identical to Step 1.

- [ ] **Step 3: Commit**

```bash
git add src/views/FloatingInput.vue
git commit -m "feat(floating-input): add ghost buttons for empty persona and dictionary"
```

---

### Task 4: Add hint modal to FloatingInput.vue template

**Files:**
- Modify: `src/views/FloatingInput.vue` (template section)

The modal is rendered once at the root level, inside the outermost `<div>` but after the `<div ref="contentWrapRef">`. It covers the viewport with a backdrop and shows a centered card.

- [ ] **Step 1: Add modal markup**

In the template, after the closing `</div>` of `<div ref="contentWrapRef">` (the one just before `</template>` at the end of the file), insert:

```vue
    <!-- Empty-state hint modal -->
    <Transition name="drop">
      <div
        v-if="emptyHintTarget"
        class="fixed inset-0 z-50 flex items-center justify-center"
        style="background: rgba(0,0,0,0.4); backdrop-filter: blur(4px)"
      >
        <div class="empty-hint-card">
          <p class="empty-hint-title" style="color: var(--color-text)">
            {{ emptyHintTarget === 'persona' ? t('floating.emptyPersonaTitle') : t('floating.emptyDictTitle') }}
          </p>
          <p class="empty-hint-body" style="color: var(--color-text-secondary)">
            {{ emptyHintTarget === 'persona' ? t('floating.emptyPersonaBody') : t('floating.emptyDictBody') }}
          </p>
          <div class="empty-hint-actions">
            <button
              class="empty-hint-cancel"
              @click="emptyHintTarget = null"
            >
              {{ t('common.cancel') }}
            </button>
            <button
              class="empty-hint-go"
              @click="handleEmptyHintGo"
            >
              {{ t('floating.goToSettings') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
```

- [ ] **Step 2: Commit**

```bash
git add src/views/FloatingInput.vue
git commit -m "feat(floating-input): add empty-state hint modal"
```

---

### Task 5: Add CSS styles for ghost buttons and modal

**Files:**
- Modify: `src/views/FloatingInput.vue` (style section)

- [ ] **Step 1: Add ghost button styles**

Append to the `<style scoped>` block, after the existing `.dict-dot` styles (around line 1046) and before the `/* Model dropdown */` comment:

```css
/* ── Ghost button (empty state) ── */
.ghost-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  width: 28px;
  border-radius: 8px;
  border: 1px dashed var(--color-border);
  background: transparent;
  color: var(--color-text-muted);
  opacity: 0.4;
  cursor: default;
  flex-shrink: 0;
}
```

- [ ] **Step 2: Add modal styles**

Append to the `<style scoped>` block, before the closing `</style>` tag:

```css
/* ── Empty-state hint modal ── */
.empty-hint-card {
  max-width: 280px;
  width: calc(100% - 48px);
  padding: 20px;
  border-radius: 14px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
}
.empty-hint-title {
  font-size: 13px;
  font-weight: 600;
  line-height: 1.4;
  margin-bottom: 6px;
}
.empty-hint-body {
  font-size: 12px;
  line-height: 1.5;
  margin-bottom: 16px;
}
.empty-hint-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.empty-hint-cancel {
  height: 30px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 550;
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  cursor: pointer;
  transition: all 0.15s ease;
}
.empty-hint-cancel:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.empty-hint-go {
  height: 30px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 550;
  color: white;
  background: var(--color-accent);
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
}
.empty-hint-go:hover {
  opacity: 0.9;
}
```

- [ ] **Step 3: Commit**

```bash
git add src/views/FloatingInput.vue
git commit -m "style(floating-input): add ghost button and hint modal styles"
```

---

### Task 6: Add scrollTo-persona support in Settings.vue

**Files:**
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: Add id to persona section**

In the template, find the persona `EditableCardList` (inside the `v-if="activeTab === 'translation'"` block). Add `id="persona-section"` to the wrapping element. Currently (around line 1058):

```vue
        <EditableCardList
          class="mt"
          :items="personaStore.personas"
```

Change to:

```vue
        <EditableCardList
          id="persona-section"
          class="mt"
          :items="personaStore.personas"
```

Note: `EditableCardList` renders a root element, so `id` will be placed on it.

- [ ] **Step 2: Add scrollTo logic in onMounted**

In the `onMounted` callback (around line 547), after the existing `route.query.tab` check:

Current code:
```typescript
  if (route.query.tab === "translation") {
    activeTab.value = "translation";
  }
```

Replace with:
```typescript
  if (route.query.tab === "translation") {
    activeTab.value = "translation";
  }
  if (route.query.scrollTo === "persona") {
    nextTick(() => {
      document.getElementById("persona-section")?.scrollIntoView({ behavior: "smooth" });
    });
  }
```

- [ ] **Step 3: Commit**

```bash
git add src/views/Settings.vue
git commit -m "feat(settings): add scrollTo=persona support for deep-linking from empty hint"
```

---

### Task 7: Visual polish with design-taste-frontend skill

**Files:**
- Modify: `src/views/FloatingInput.vue` (CSS refinements)

- [ ] **Step 1: Run the app and verify the feature**

Run the Tauri dev server and test:
1. With empty persona/dict — ghost buttons should appear in toolbar
2. Click ghost button — modal should appear with backdrop blur
3. Click "Go to Settings" — should navigate to correct page
4. With entries present — ghost buttons should NOT appear, normal toggle buttons should appear
5. Both growAbove and normal modes should work

- [ ] **Step 2: Invoke design-taste-frontend skill for polish**

Invoke the `design-taste-frontend` skill. Pass context: the ghost button and modal styles added in Tasks 3–5. Ask it to review and refine:
- Ghost button visual treatment (opacity, border style, sizing)
- Modal card spacing, typography, button proportions
- Transition timing and easing
- Dark/light mode consistency with existing `var(--color-*)` tokens

Apply any refinements suggested by the skill.

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "style(floating-input): polish empty-state ghost buttons and hint modal"
```
