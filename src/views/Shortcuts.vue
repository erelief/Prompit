<script setup lang="ts">
// Shortcuts page — every keyboard / keyboard+mouse operation in the app on one
// screen, grouped by function. Only the six configurable bindings are editable
// (same useShortcutRecorder UI that used to live in Settings → General →
// System); the built-in ones are display-only kbd badges.
import { computed, type Component } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import {
  ArrowLeft,
  Globe,
  AppWindow,
  Pencil,
  Settings2,
  Sparkles,
  Keyboard,
  RotateCcw,
  ChevronUp,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
} from "@lucide/vue";
import { appConfig } from "../stores/config";
import { altKey, ctrlKey } from "../utils/platform";
import { shortcutsEqual } from "../utils/shortcut";
import { useShortcutRecorder } from "../composables/useShortcutRecorder";
import { useSettingsWindow } from "../composables/useSettingsWindow";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

// ── Shortcut recorders (wake-up + mode-switch + forward-to-input + edit-result
//    + skills-prev + skills-next) ──
// All six share the same UI/validate/conflict logic via useShortcutRecorder.
// The wake shortcut is an OS-global hotkey re-registered through Tauri; the
// others are webview-scoped and just write the config field. Each recorder
// passes every other field for conflict detection so no two bindings collide.
const wakeField = computed({ get: () => appConfig.shortcut, set: (v) => { appConfig.shortcut = v; } });
const modeField = computed({ get: () => appConfig.mode_shortcut, set: (v) => { appConfig.mode_shortcut = v; } });
const forwardField = computed({ get: () => appConfig.forward_shortcut, set: (v) => { appConfig.forward_shortcut = v; } });
const editField = computed({ get: () => appConfig.edit_shortcut, set: (v) => { appConfig.edit_shortcut = v; } });
const skillsPrevField = computed({ get: () => appConfig.skills_prev_shortcut, set: (v) => { appConfig.skills_prev_shortcut = v; } });
const skillsNextField = computed({ get: () => appConfig.skills_next_shortcut, set: (v) => { appConfig.skills_next_shortcut = v; } });

const {
  recording: shortcutRecording, error: shortcutError, recBtn: shortcutRecBtn,
  tokens: shortcutTokens, start: startShortcutRecord, cancel: cancelShortcutRecord,
  onKeydown: onShortcutKeydown, reset: resetShortcut,
} = useShortcutRecorder(t, {
  field: wakeField, otherFields: [modeField, forwardField, editField, skillsPrevField, skillsNextField],
  defaultBinding: "Alt+Y",
  invalidMsg: "settings.shortcutInvalid", conflictMsg: "settings.shortcutConflict",
  tauriGlobal: true,
});

const {
  recording: modeShortcutRecording, error: modeShortcutError, recBtn: modeShortcutRecBtn,
  tokens: modeShortcutTokens, start: startModeShortcutRecord, cancel: cancelModeShortcutRecord,
  onKeydown: onModeShortcutKeydown, reset: resetModeShortcut,
} = useShortcutRecorder(t, {
  field: modeField, otherFields: [wakeField, forwardField, editField, skillsPrevField, skillsNextField],
  defaultBinding: "Alt+M",
  invalidMsg: "settings.shortcutInvalid", conflictMsg: "settings.shortcutConflict",
});

const {
  recording: forwardShortcutRecording, error: forwardShortcutError, recBtn: forwardShortcutRecBtn,
  tokens: forwardShortcutTokens, start: startForwardShortcutRecord, cancel: cancelForwardShortcutRecord,
  onKeydown: onForwardShortcutKeydown, reset: resetForwardShortcut,
} = useShortcutRecorder(t, {
  field: forwardField, otherFields: [wakeField, modeField, editField, skillsPrevField, skillsNextField],
  defaultBinding: "Alt+F",
  invalidMsg: "settings.shortcutInvalid", conflictMsg: "settings.shortcutConflict",
});

const {
  recording: editShortcutRecording, error: editShortcutError, recBtn: editShortcutRecBtn,
  tokens: editShortcutTokens, start: startEditShortcutRecord, cancel: cancelEditShortcutRecord,
  onKeydown: onEditShortcutKeydown, reset: resetEditShortcut,
} = useShortcutRecorder(t, {
  field: editField, otherFields: [wakeField, modeField, forwardField, skillsPrevField, skillsNextField],
  defaultBinding: "Alt+E",
  invalidMsg: "settings.shortcutInvalid", conflictMsg: "settings.shortcutConflict",
});

const {
  recording: skillsPrevShortcutRecording, error: skillsPrevShortcutError, recBtn: skillsPrevShortcutRecBtn,
  tokens: skillsPrevShortcutTokens, start: startSkillsPrevShortcutRecord, cancel: cancelSkillsPrevShortcutRecord,
  onKeydown: onSkillsPrevShortcutKeydown, reset: resetSkillsPrevShortcut,
} = useShortcutRecorder(t, {
  field: skillsPrevField, otherFields: [wakeField, modeField, forwardField, editField, skillsNextField],
  defaultBinding: "Alt+Up",
  invalidMsg: "settings.shortcutInvalid", conflictMsg: "settings.shortcutConflict",
});

const {
  recording: skillsNextShortcutRecording, error: skillsNextShortcutError, recBtn: skillsNextShortcutRecBtn,
  tokens: skillsNextShortcutTokens, start: startSkillsNextShortcutRecord, cancel: cancelSkillsNextShortcutRecord,
  onKeydown: onSkillsNextShortcutKeydown, reset: resetSkillsNextShortcut,
} = useShortcutRecorder(t, {
  field: skillsNextField, otherFields: [wakeField, modeField, forwardField, editField, skillsPrevField],
  defaultBinding: "Alt+Down",
  invalidMsg: "settings.shortcutInvalid", conflictMsg: "settings.shortcutConflict",
});

// 方向键 token 渲染为 chevron 图标，其余 token（Alt/Ctrl/Shift/Cmd/字母/数字/F 键）显示文字。
const ARROW_ICON: Record<string, Component> = {
  Up: ChevronUp, Down: ChevronDown, Left: ChevronLeft, Right: ChevronRight,
};

// ── Built-in (read-only) operations ──
// These are hardcoded in FloatingInput.vue / Settings.vue handlers, so the
// list is static data; tokens render with the same .kbd-badge style as the
// editable rows. Modifier names follow the platform (Ctrl/Cmd, Alt/Option).
// Ctrl+Z rows rely on the textareas' native per-element undo stacks — neither
// handleKeydown nor handleEditKeydown intercepts the combo.
interface StaticShortcut {
  labelKey: string;
  tokens: string[];
}

// Font size is app-wide (appConfig.font_size drives the global --font-scale),
// so its wheel shortcut belongs to the Global group even though the gesture
// is performed over the floating window.
const globalStatics = computed<StaticShortcut[]>(() => [
  { labelKey: "shortcuts.fontSizeWheel", tokens: [altKey(), t("shortcuts.wheel")] },
]);

const floatingStatics = computed<StaticShortcut[]>(() => [
  { labelKey: "shortcuts.send", tokens: ["Enter"] },
  { labelKey: "shortcuts.newline", tokens: ["Shift", "Enter"] },
  { labelKey: "shortcuts.escapeStaged", tokens: ["Esc"] },
  { labelKey: "shortcuts.historyNav", tokens: ["Up", "Down"] },
  { labelKey: "shortcuts.copyResult", tokens: [ctrlKey(), "C"] },
  { labelKey: "shortcuts.undo", tokens: [ctrlKey(), "Z"] },
  { labelKey: "shortcuts.opacityWheel", tokens: [ctrlKey(), t("shortcuts.wheel")] },
]);

const editingStatics = computed<StaticShortcut[]>(() => [
  { labelKey: "shortcuts.editConfirm", tokens: ["Enter"] },
  { labelKey: "shortcuts.newline", tokens: ["Shift", "Enter"] },
  { labelKey: "shortcuts.editCancel", tokens: ["Esc"] },
  { labelKey: "shortcuts.undo", tokens: [ctrlKey(), "Z"] },
]);

const settingsStatics = computed<StaticShortcut[]>(() => [
  { labelKey: "shortcuts.undo", tokens: [ctrlKey(), "Z"] },
]);
</script>

<template>
  <div class="shortcuts-root" :class="{ 'grow-above': growAbove }">
    <!-- Header -->
    <div class="shortcuts-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('shortcuts.title') }}</span>
    </div>

    <div class="shortcuts-body">
      <!-- Global -->
      <div class="section-head">
        <span class="section-title"><Globe :size="13" />{{ t('shortcuts.groupGlobal') }}</span>
      </div>
      <div class="card-section">
        <!-- Shortcut (record a new global hotkey) -->
        <div class="card-row shortcut-row">
          <span class="card-label">{{ t('settings.shortcut') }}</span>
          <div class="shortcut-controls">
            <button
              ref="shortcutRecBtn"
              class="shortcut-btn"
              :class="{ recording: shortcutRecording, 'has-error': !!shortcutError }"
              :title="t('settings.shortcutHint')"
              tabindex="0"
              @click="shortcutRecording ? cancelShortcutRecord() : startShortcutRecord()"
              @keydown="onShortcutKeydown"
              @blur="cancelShortcutRecord"
            >
              <Keyboard :size="13" class="shortcut-btn-icon" :stroke-width="1.8" />
              <template v-if="shortcutRecording">
                <span class="shortcut-rec-text">{{ t('settings.shortcutRecording') }}</span>
              </template>
              <template v-else-if="shortcutError">
                <span class="shortcut-err-text">{{ shortcutError }}</span>
              </template>
              <template v-else>
                <kbd v-for="(tok, i) in shortcutTokens" :key="i" class="kbd-badge">
                  <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
                  <template v-else>{{ tok }}</template>
                </kbd>
              </template>
            </button>
            <button
              class="shortcut-reset"
              :class="{ 'shortcut-reset-off': shortcutsEqual('Alt+Y', appConfig.shortcut) }"
              :disabled="shortcutsEqual('Alt+Y', appConfig.shortcut)"
              @click="resetShortcut"
              :title="t('settings.resetToDefault')"
            >
              <RotateCcw :size="11" :stroke-width="2" />
            </button>
          </div>
        </div>
        <!-- Built-in global operations (read-only) -->
        <div v-for="item in globalStatics" :key="item.labelKey" class="card-row">
          <span class="card-label">{{ t(item.labelKey) }}</span>
          <div class="kbd-static">
            <kbd v-for="(tok, i) in item.tokens" :key="i" class="kbd-badge">
              <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
              <template v-else>{{ tok }}</template>
            </kbd>
          </div>
        </div>
      </div>

      <!-- Floating input -->
      <div class="section-head mt">
        <span class="section-title"><AppWindow :size="13" />{{ t('shortcuts.groupFloating') }}</span>
      </div>
      <div class="card-section">
        <!-- Mode-switch shortcut (webview-scoped, active only in FloatingInput) -->
        <div class="card-row shortcut-row">
          <span class="card-label">{{ t('settings.modeShortcut') }}</span>
          <div class="shortcut-controls">
            <button
              ref="modeShortcutRecBtn"
              class="shortcut-btn"
              :class="{ recording: modeShortcutRecording, 'has-error': !!modeShortcutError }"
              :title="t('settings.modeShortcutHint')"
              tabindex="0"
              @click="modeShortcutRecording ? cancelModeShortcutRecord() : startModeShortcutRecord()"
              @keydown="onModeShortcutKeydown"
              @blur="cancelModeShortcutRecord"
            >
              <Keyboard :size="13" class="shortcut-btn-icon" :stroke-width="1.8" />
              <template v-if="modeShortcutRecording">
                <span class="shortcut-rec-text">{{ t('settings.shortcutRecording') }}</span>
              </template>
              <template v-else-if="modeShortcutError">
                <span class="shortcut-err-text">{{ modeShortcutError }}</span>
              </template>
              <template v-else>
                <kbd v-for="(tok, i) in modeShortcutTokens" :key="i" class="kbd-badge">
                  <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
                  <template v-else>{{ tok }}</template>
                </kbd>
              </template>
            </button>
            <button
              class="shortcut-reset"
              :class="{ 'shortcut-reset-off': shortcutsEqual('Alt+M', appConfig.mode_shortcut) }"
              :disabled="shortcutsEqual('Alt+M', appConfig.mode_shortcut)"
              @click="resetModeShortcut"
              :title="t('settings.resetToDefault')"
            >
              <RotateCcw :size="11" :stroke-width="2" />
            </button>
          </div>
        </div>
        <!-- Send-to-input shortcut (webview-scoped, active only in FloatingInput) -->
        <div class="card-row shortcut-row">
          <span class="card-label">{{ t('settings.forwardShortcut') }}</span>
          <div class="shortcut-controls">
            <button
              ref="forwardShortcutRecBtn"
              class="shortcut-btn"
              :class="{ recording: forwardShortcutRecording, 'has-error': !!forwardShortcutError }"
              :title="t('settings.forwardShortcutHint')"
              tabindex="0"
              @click="forwardShortcutRecording ? cancelForwardShortcutRecord() : startForwardShortcutRecord()"
              @keydown="onForwardShortcutKeydown"
              @blur="cancelForwardShortcutRecord"
            >
              <Keyboard :size="13" class="shortcut-btn-icon" :stroke-width="1.8" />
              <template v-if="forwardShortcutRecording">
                <span class="shortcut-rec-text">{{ t('settings.shortcutRecording') }}</span>
              </template>
              <template v-else-if="forwardShortcutError">
                <span class="shortcut-err-text">{{ forwardShortcutError }}</span>
              </template>
              <template v-else>
                <kbd v-for="(tok, i) in forwardShortcutTokens" :key="i" class="kbd-badge">
                  <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
                  <template v-else>{{ tok }}</template>
                </kbd>
              </template>
            </button>
            <button
              class="shortcut-reset"
              :class="{ 'shortcut-reset-off': shortcutsEqual('Alt+F', appConfig.forward_shortcut) }"
              :disabled="shortcutsEqual('Alt+F', appConfig.forward_shortcut)"
              @click="resetForwardShortcut"
              :title="t('settings.resetToDefault')"
            >
              <RotateCcw :size="11" :stroke-width="2" />
            </button>
          </div>
        </div>
        <!-- Edit-result shortcut (webview-scoped, active only in FloatingInput) -->
        <div class="card-row shortcut-row">
          <span class="card-label">{{ t('settings.editShortcut') }}</span>
          <div class="shortcut-controls">
            <button
              ref="editShortcutRecBtn"
              class="shortcut-btn"
              :class="{ recording: editShortcutRecording, 'has-error': !!editShortcutError }"
              :title="t('settings.editShortcutHint')"
              tabindex="0"
              @click="editShortcutRecording ? cancelEditShortcutRecord() : startEditShortcutRecord()"
              @keydown="onEditShortcutKeydown"
              @blur="cancelEditShortcutRecord"
            >
              <Keyboard :size="13" class="shortcut-btn-icon" :stroke-width="1.8" />
              <template v-if="editShortcutRecording">
                <span class="shortcut-rec-text">{{ t('settings.shortcutRecording') }}</span>
              </template>
              <template v-else-if="editShortcutError">
                <span class="shortcut-err-text">{{ editShortcutError }}</span>
              </template>
              <template v-else>
                <kbd v-for="(tok, i) in editShortcutTokens" :key="i" class="kbd-badge">
                  <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
                  <template v-else>{{ tok }}</template>
                </kbd>
              </template>
            </button>
            <button
              class="shortcut-reset"
              :class="{ 'shortcut-reset-off': shortcutsEqual('Alt+E', appConfig.edit_shortcut) }"
              :disabled="shortcutsEqual('Alt+E', appConfig.edit_shortcut)"
              @click="resetEditShortcut"
              :title="t('settings.resetToDefault')"
            >
              <RotateCcw :size="11" :stroke-width="2" />
            </button>
          </div>
        </div>
        <!-- Built-in floating-window operations (read-only) -->
        <div v-for="item in floatingStatics" :key="item.labelKey" class="card-row">
          <span class="card-label">{{ t(item.labelKey) }}</span>
          <div class="kbd-static">
            <kbd v-for="(tok, i) in item.tokens" :key="i" class="kbd-badge">
              <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
              <template v-else>{{ tok }}</template>
            </kbd>
          </div>
        </div>
      </div>

      <!-- Skills Lite -->
      <div class="section-head mt">
        <span class="section-title"><Sparkles :size="13" />{{ t('modes.skillsLite') }}</span>
      </div>
      <div class="card-section">
        <!-- Skills-prev shortcut (webview-scoped, skills_lite mode only) -->
        <div class="card-row shortcut-row">
          <span class="card-label">{{ t('settings.skillsPrevShortcut') }}</span>
          <div class="shortcut-controls">
            <button
              ref="skillsPrevShortcutRecBtn"
              class="shortcut-btn"
              :class="{ recording: skillsPrevShortcutRecording, 'has-error': !!skillsPrevShortcutError }"
              :title="t('settings.skillsPrevShortcutHint')"
              tabindex="0"
              @click="skillsPrevShortcutRecording ? cancelSkillsPrevShortcutRecord() : startSkillsPrevShortcutRecord()"
              @keydown="onSkillsPrevShortcutKeydown"
              @blur="cancelSkillsPrevShortcutRecord"
            >
              <Keyboard :size="13" class="shortcut-btn-icon" :stroke-width="1.8" />
              <template v-if="skillsPrevShortcutRecording">
                <span class="shortcut-rec-text">{{ t('settings.shortcutRecording') }}</span>
              </template>
              <template v-else-if="skillsPrevShortcutError">
                <span class="shortcut-err-text">{{ skillsPrevShortcutError }}</span>
              </template>
              <template v-else>
                <kbd v-for="(tok, i) in skillsPrevShortcutTokens" :key="i" class="kbd-badge">
                  <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
                  <template v-else>{{ tok }}</template>
                </kbd>
              </template>
            </button>
            <button
              class="shortcut-reset"
              :class="{ 'shortcut-reset-off': shortcutsEqual('Alt+Up', appConfig.skills_prev_shortcut) }"
              :disabled="shortcutsEqual('Alt+Up', appConfig.skills_prev_shortcut)"
              @click="resetSkillsPrevShortcut"
              :title="t('settings.resetToDefault')"
            >
              <RotateCcw :size="11" :stroke-width="2" />
            </button>
          </div>
        </div>
        <!-- Skills-next shortcut (webview-scoped, skills_lite mode only) -->
        <div class="card-row shortcut-row">
          <span class="card-label">{{ t('settings.skillsNextShortcut') }}</span>
          <div class="shortcut-controls">
            <button
              ref="skillsNextShortcutRecBtn"
              class="shortcut-btn"
              :class="{ recording: skillsNextShortcutRecording, 'has-error': !!skillsNextShortcutError }"
              :title="t('settings.skillsNextShortcutHint')"
              tabindex="0"
              @click="skillsNextShortcutRecording ? cancelSkillsNextShortcutRecord() : startSkillsNextShortcutRecord()"
              @keydown="onSkillsNextShortcutKeydown"
              @blur="cancelSkillsNextShortcutRecord"
            >
              <Keyboard :size="13" class="shortcut-btn-icon" :stroke-width="1.8" />
              <template v-if="skillsNextShortcutRecording">
                <span class="shortcut-rec-text">{{ t('settings.shortcutRecording') }}</span>
              </template>
              <template v-else-if="skillsNextShortcutError">
                <span class="shortcut-err-text">{{ skillsNextShortcutError }}</span>
              </template>
              <template v-else>
                <kbd v-for="(tok, i) in skillsNextShortcutTokens" :key="i" class="kbd-badge">
                  <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
                  <template v-else>{{ tok }}</template>
                </kbd>
              </template>
            </button>
            <button
              class="shortcut-reset"
              :class="{ 'shortcut-reset-off': shortcutsEqual('Alt+Down', appConfig.skills_next_shortcut) }"
              :disabled="shortcutsEqual('Alt+Down', appConfig.skills_next_shortcut)"
              @click="resetSkillsNextShortcut"
              :title="t('settings.resetToDefault')"
            >
              <RotateCcw :size="11" :stroke-width="2" />
            </button>
          </div>
        </div>
      </div>

      <!-- Editing result -->
      <div class="section-head mt">
        <span class="section-title"><Pencil :size="13" />{{ t('shortcuts.groupEditing') }}</span>
      </div>
      <div class="card-section">
        <div v-for="item in editingStatics" :key="item.labelKey" class="card-row">
          <span class="card-label">{{ t(item.labelKey) }}</span>
          <div class="kbd-static">
            <kbd v-for="(tok, i) in item.tokens" :key="i" class="kbd-badge">
              <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
              <template v-else>{{ tok }}</template>
            </kbd>
          </div>
        </div>
      </div>

      <!-- Settings page -->
      <div class="section-head mt">
        <span class="section-title"><Settings2 :size="13" />{{ t('shortcuts.groupSettings') }}</span>
      </div>
      <div class="card-section">
        <div v-for="item in settingsStatics" :key="item.labelKey" class="card-row">
          <span class="card-label">{{ t(item.labelKey) }}</span>
          <div class="kbd-static">
            <kbd v-for="(tok, i) in item.tokens" :key="i" class="kbd-badge">
              <component v-if="ARROW_ICON[tok]" :is="ARROW_ICON[tok]" :size="11" :stroke-width="2" />
              <template v-else>{{ tok }}</template>
            </kbd>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Shortcuts — same layout & card vocabulary as Settings.vue
   ══════════════════════════════════════ */
.shortcuts-root {
  height: calc(100dvh / var(--font-scale, 1)); display: flex; flex-direction: column;
  background: var(--color-bg); color: var(--color-text); overflow: hidden;
  border-radius: 11px;
}
.shortcuts-root.grow-above .shortcuts-header { order: 2; border-bottom: none; border-top: 1px solid var(--color-surface); }
.shortcuts-root.grow-above .shortcuts-body { order: 0; }

/* ── Header ── */
.shortcuts-header {
  display: flex; align-items: center; gap: 12px;
  padding: 16px 24px 12px; border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}

/* ── Body scroll ── */
.shortcuts-body {
  flex: 1; overflow-y: auto; padding: 10px 24px 16px;
}
.shortcuts-body::-webkit-scrollbar{width:3px}
.shortcuts-body::-webkit-scrollbar-thumb{background:var(--color-scrollbar);border-radius:3px}

/* ── Section head ── */
.section-head {
  display:flex; align-items:center; justify-content:space-between;
  margin-bottom: 10px;
}
.section-head.mt { margin-top: 18px; }
.section-title {
  display:flex; align-items:center; gap:7px;
  font-size: 11.5px; font-weight: 650; letter-spacing: .01em;
  color: var(--color-text-secondary);
}

/* ── Card section: reusable grouped-settings container ── */
.card-section {
  display: flex; flex-direction: column; gap: 8px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 10px;
  padding: 12px 14px;
}
.card-row {
  display: flex; align-items: center; justify-content: space-between;
  gap: 12px;
}
.card-row + .card-row {
  border-top: 1px solid var(--color-border);
  padding-top: 8px;
}
.card-label {
  font-size: 11px; font-weight: 500;
  color: var(--color-text-muted);
  white-space: nowrap; flex-shrink: 0;
}

/* ── Shortcut recorder ── */
.shortcut-controls {
  display: flex;
  align-items: center;
  gap: 4px;
}
.shortcut-reset {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: 0.15s;
}
.shortcut-reset:not(:disabled):hover {
  background: var(--color-border);
  color: var(--color-text-secondary);
}
.shortcut-reset-off {
  opacity: 0.4;
  cursor: not-allowed;
}
.shortcut-row .shortcut-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 26px;
  padding: 0 7px;
  border-radius: 6px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 11px;
  font-family: inherit;
  transition: 0.15s;
  outline: none;
}
.shortcut-btn:hover {
  border-color: var(--color-border-hover);
  background: var(--color-surface-hover);
}
.shortcut-btn:focus-visible,
.shortcut-btn.recording {
  border-color: var(--color-accent);
  background: var(--color-accent-bg);
}
.shortcut-btn-icon {
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.shortcut-rec-text {
  color: var(--color-accent-text);
  font-weight: 500;
}
.shortcut-err-text {
  color: var(--color-danger);
  font-weight: 500;
}
.kbd-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 18px;
  padding: 0 5px;
  border-radius: 4px;
  background: var(--color-overlay);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  font-family: inherit;
}

/* ── Read-only bindings: same badges, no button chrome ── */
.kbd-static {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 26px;
  padding: 0 7px;
}
</style>
