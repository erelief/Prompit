// Reusable shortcut-string recorder for the Settings page.
//
// Both the wake-up shortcut (OS-global, Tauri-registered) and the mode-switch
// shortcut (webview-scoped) share the same UI: a button that enters a
// "press a combo" state, validates it, rejects conflicts with the *other*
// shortcut, and auto-dismisses errors. The only real differences are which
// config field is read/written and whether a Rust command re-registers the
// binding. Those are captured by the options below.

import { ref, computed, type WritableComputedRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { nextTick } from "vue";
import {
  buildShortcutFromEvent,
  parseShortcutTokens,
  shortcutsEqual,
} from "../utils/shortcut";
import { burstParticles } from "../utils/burstParticles";

export interface ShortcutRecorderOptions {
  /** The reactive config field holding the current binding string. */
  field: WritableComputedRef<string>;
  /** The *other* shortcuts' reactive fields, for conflict detection.
   *  Pass every shortcut field except the recorder's own so a candidate
   *  is rejected if it collides with any other binding. */
  otherFields: WritableComputedRef<string>[];
  /** Factory default to restore on Backspace. */
  defaultBinding: string;
  /** i18n key for the "needs a modifier" message. */
  invalidMsg: string;
  /** i18n key for the "conflicts with the other shortcut" message. */
  conflictMsg: string;
  /** When true, the binding is re-registered via Tauri on apply
   *  (`update_shortcut`) and raw key presses are captured via
   *  `start_record_shortcut`/`finish_record_shortcut`. False for the
   *  webview-scoped mode shortcut, which just writes the field. */
  tauriGlobal?: boolean;
}

export function useShortcutRecorder(t: (key: string) => string, opts: ShortcutRecorderOptions) {
  const recording = ref(false);
  const error = ref("");
  const recBtn = ref<HTMLButtonElement | null>(null);
  let errorTimer: ReturnType<typeof setTimeout> | null = null;
  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  const tokens = computed(() => parseShortcutTokens(opts.field.value));

  function showError(msg: string) {
    recording.value = false;
    if (opts.tauriGlobal && isTauri) invoke("finish_record_shortcut").catch(() => {});
    if (errorTimer) clearTimeout(errorTimer);
    error.value = msg;
    errorTimer = setTimeout(() => {
      error.value = "";
      errorTimer = null;
    }, 1800);
  }

  function clearError() {
    if (errorTimer) { clearTimeout(errorTimer); errorTimer = null; }
    error.value = "";
  }

  async function apply(binding: string) {
    clearError();
    if (opts.tauriGlobal) {
      if (!isTauri) return;
      try {
        await invoke("update_shortcut", { shortcut: binding });
        opts.field.value = binding; // existing watcher persists config.json
        if (recBtn.value) burstParticles(recBtn.value);
      } catch {
        showError(t(opts.conflictMsg));
        // update_shortcut already rolled back; nothing to restore.
      }
    } else {
      opts.field.value = binding;
      clearError();
      if (recBtn.value) burstParticles(recBtn.value);
    }
  }

  function start() {
    if (opts.tauriGlobal && !isTauri) return;
    clearError();
    recording.value = true;
    if (opts.tauriGlobal) invoke("start_record_shortcut").catch(() => {});
    nextTick(() => recBtn.value?.focus());
  }

  async function cancel() {
    if (!recording.value) return;
    recording.value = false;
    clearError();
    if (opts.tauriGlobal && isTauri) await invoke("finish_record_shortcut").catch(() => {});
  }

  async function onKeydown(e: KeyboardEvent) {
    if (!recording.value) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.code === "Escape") { await cancel(); return; }
    if (e.code === "Backspace") {
      recording.value = false;
      await apply(opts.defaultBinding);
      return;
    }

    const candidate = buildShortcutFromEvent(e);
    // Modifier-only presses carry no main key token: stay quiet.
    if (!candidate) {
      // A real key was pressed but no modifier is held → invalid.
      if (!e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
        showError(t(opts.invalidMsg));
      }
      return;
    }
    if (opts.otherFields.some((f) => shortcutsEqual(candidate, f.value))) {
      showError(t(opts.conflictMsg));
      return;
    }
    recording.value = false;
    await apply(candidate);
  }

  /** Restore the factory default; refuses if another shortcut holds it. */
  async function reset() {
    const def = opts.defaultBinding;
    if (shortcutsEqual(def, opts.field.value)) return;
    if (opts.otherFields.some((f) => shortcutsEqual(def, f.value))) {
      showError(t(opts.conflictMsg));
      return;
    }
    recording.value = false;
    await apply(def);
  }

  return { recording, error, recBtn, tokens, start, cancel, onKeydown, reset };
}
