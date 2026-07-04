import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useIntervalFn } from "@vueuse/core";

/**
 * Shared state + behavior for the "import encrypted backup" flow.
 *
 * Extracted so the Settings page (`UserData.vue`) and the onboarding import
 * page (`OnboardingImport.vue`) share ONE source of truth for the import logic
 * — file pick, password, 5-second countdown confirm, IPC, status reporting.
 * Any change here applies to both pages automatically; the pages only own their
 * visual style, copy, and what happens after a successful import (via the
 * `onSuccess` callback, which defaults to the legacy "show restart hint").
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
  /** Invoked after a successful `import_data`. Default: no-op (caller sets a
   *  status message). The onboarding page overrides this to refresh in-memory
   *  state and jump to the "ready to use" screen without a restart. */
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

  const importTimer = useIntervalFn(() => {
    if (importCountdown.value > 0) importCountdown.value--;
    else importTimer.pause();
  }, 1000, { immediate: false });

  const importFileName = computed(() => {
    if (!importPath.value) return "";
    const parts = importPath.value.replace(/\\/g, "/").split("/");
    return parts[parts.length - 1] || importPath.value;
  });

  const importCanConfirm = computed(
    () => !!importPath.value && importPassword.value.length > 0 && !importConfirming.value,
  );

  function stopCountdown() {
    importConfirming.value = false;
    importCountdown.value = CONFIRM_COUNTDOWN_SECONDS;
    importTimer.pause();
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
    stopCountdown();
    importStatus.value = { kind: "idle", msg: "" };
  }

  function resetImport() {
    importPath.value = null;
    importPassword.value = "";
    importShowPw.value = false;
    stopCountdown();
  }

  function requestImport() {
    if (!importCanConfirm.value) return;
    importConfirming.value = true;
    importCountdown.value = CONFIRM_COUNTDOWN_SECONDS;
    importTimer.resume();
  }

  async function confirmImport() {
    if (importCountdown.value > 0 || !importPath.value) return;
    importBusy.value = true;
    try {
      await invoke("import_data", {
        path: importPath.value,
        password: importPassword.value,
      });
      if (opts.onSuccess) {
        await opts.onSuccess();
      } else {
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
    // derived
    importFileName,
    importCanConfirm,
    // actions
    selectImportFile,
    requestImport,
    confirmImport,
    stopCountdown,
    resetImport,
  };
}
