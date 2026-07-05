import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useIntervalFn } from "@vueuse/core";
import { loadConfig } from "../stores/config";
import type { BundlePreview, CategoryPreview } from "./useDataCategories";

/**
 * Shared state + behavior for the "import encrypted backup" flow.
 *
 * Flow: pick file → enter password → analyze (inspect_bundle) → select
 * categories → 5-second countdown confirm → import (import_data) → hot-reload
 * config in-place. Extracted so the Settings import page and the onboarding
 * import step share ONE source of truth. Callers own visual style, copy, and
 * may override `onSuccess` (the onboarding page routes to its summary step
 * instead of relying on the default status message).
 *
 * The Status shape and the 5-second confirm-with-countdown pattern are shared
 * with the export path and the reset path elsewhere in the app.
 */
export type ImportStatusKind = "idle" | "info" | "success" | "error";
export interface ImportStatus {
  kind: ImportStatusKind;
  msg: string;
}

const JSON_FILTER = [{ name: "JSON", extensions: ["json"] }];
const CONFIRM_COUNTDOWN_SECONDS = 5;

export interface UseDataImportOptions {
  /** i18n message keys (resolved by the caller via t()) for status strings. */
  messages: {
    cancelled: string;
    success: string;
    error: (message: string) => string;
  };
  /** Invoked after a successful `import_data`. Default: hot-reload config and
   *  set a success status. The onboarding page overrides this to reload config
   *  and advance to the post-import summary step. */
  onSuccess?: () => void | Promise<void>;
}

export function useDataImport(opts: UseDataImportOptions) {
  const importPath = ref<string | null>(null);
  const importPassword = ref("");
  const importShowPw = ref(false);
  const importConfirming = ref(false);
  const importCountdown = ref(CONFIRM_COUNTDOWN_SECONDS);
  const importStatus = ref<ImportStatus>({ kind: "idle", msg: "" });
  const importBusy = ref(false);

  // Analyze step: after entering a password the user clicks Analyze, which
  // calls inspect_bundle. The returned preview drives the category selector.
  const importPreview = ref<CategoryPreview[]>([]);
  const importSelected = ref<Set<string>>(new Set());
  const importAnalyzed = ref(false);
  const importAnalyzing = ref(false);

  const importTimer = useIntervalFn(() => {
    if (importCountdown.value > 0) importCountdown.value--;
    else importTimer.pause();
  }, 1000, { immediate: false });

  const importFileName = computed(() => {
    if (!importPath.value) return "";
    const parts = importPath.value.replace(/\\/g, "/").split("/");
    return parts[parts.length - 1] || importPath.value;
  });

  // Analyze requires a path + a non-empty password (the bundle is password-
  // encrypted, so even inspection needs the password to unwrap the Master Key).
  const importCanAnalyze = computed(
    () => !!importPath.value && importPassword.value.length > 0 && !importAnalyzing.value,
  );

  // Confirm requires analysis done + at least one category selected.
  const importCanConfirm = computed(
    () => importAnalyzed.value
      && importSelected.value.size > 0
      && !importConfirming.value
      && !importAnalyzing.value,
  );

  function stopCountdown() {
    importConfirming.value = false;
    importCountdown.value = CONFIRM_COUNTDOWN_SECONDS;
    importTimer.pause();
  }

  function clearAnalyzed() {
    importAnalyzed.value = false;
    importPreview.value = [];
    importSelected.value = new Set();
  }

  async function selectImportFile() {
    const selected = await open({ multiple: false, filters: JSON_FILTER });
    const path = typeof selected === "string" ? selected : null;
    if (!path) {
      importStatus.value = { kind: "info", msg: opts.messages.cancelled };
      return;
    }
    importPath.value = path;
    importPassword.value = "";
    clearAnalyzed();
    stopCountdown();
    importStatus.value = { kind: "idle", msg: "" };
  }

  function resetImport() {
    importPath.value = null;
    importPassword.value = "";
    importShowPw.value = false;
    clearAnalyzed();
    stopCountdown();
  }

  async function analyzeImport() {
    if (!importCanAnalyze.value) return;
    importAnalyzing.value = true;
    importStatus.value = { kind: "idle", msg: "" };
    try {
      const preview = await invoke<BundlePreview>("inspect_bundle", {
        path: importPath.value,
        password: importPassword.value,
      });
      importPreview.value = preview.categories;
      // Default to importing everything present in the bundle.
      importSelected.value = new Set(preview.categories.map((c) => c.id));
      importAnalyzed.value = true;
    } catch (err) {
      importStatus.value = { kind: "error", msg: opts.messages.error(String(err)) };
      clearAnalyzed();
    } finally {
      importAnalyzing.value = false;
    }
  }

  function requestImport() {
    if (!importCanConfirm.value) return;
    importConfirming.value = true;
    importCountdown.value = CONFIRM_COUNTDOWN_SECONDS;
    importTimer.resume();
  }

  async function confirmImport() {
    if (importCountdown.value > 0 || !importPath.value || !importAnalyzed.value) return;
    importBusy.value = true;
    try {
      await invoke("import_data", {
        path: importPath.value,
        password: importPassword.value,
        categories: [...importSelected.value],
      });
      // Default hot-reload: re-hydrate config + encrypted stores so the UI
      // reflects the imported data without a restart. Callers that override
      // onSuccess (onboarding) do their own loadConfig + routing.
      if (opts.onSuccess) {
        await opts.onSuccess();
      } else {
        await loadConfig();
        importStatus.value = { kind: "success", msg: opts.messages.success };
      }
      resetImport();
    } catch (err) {
      importStatus.value = {
        kind: "error",
        msg: opts.messages.error(String(err)),
      };
      stopCountdown();
    } finally {
      importBusy.value = false;
    }
  }

  return {
    // state
    importPath,
    importPassword,
    importShowPw,
    importConfirming,
    importCountdown,
    importStatus,
    importBusy,
    importPreview,
    importSelected,
    importAnalyzed,
    importAnalyzing,
    // derived
    importFileName,
    importCanAnalyze,
    importCanConfirm,
    // actions
    selectImportFile,
    analyzeImport,
    requestImport,
    confirmImport,
    stopCountdown,
    resetImport,
  };
}
