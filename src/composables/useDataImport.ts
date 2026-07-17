import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useIntervalFn } from "@vueuse/core";
import { loadConfig } from "../stores/config";
import type { BundlePreview, CategoryPreview } from "./useDataCategories";

/**
 * Shared state + behavior for the "restore from encrypted backup" flow.
 *
 * Flow: pick a source (local backup file, or a backup file on the configured
 * WebDAV server) → enter password → analyze (inspect) → select categories →
 * 5-second countdown confirm → restore → hot-reload config in-place.
 * Extracted so the Settings restore page and the onboarding restore step share
 * ONE source of truth. Callers own visual style, copy, and may override
 * `onSuccess` (the onboarding page routes to its summary step instead of
 * relying on the default status message).
 *
 * The Status shape and the 5-second confirm-with-countdown pattern are shared
 * with the backup path and the reset path elsewhere in the app.
 */
export type ImportStatusKind = "idle" | "info" | "success" | "error";
export interface ImportStatus {
  kind: ImportStatusKind;
  msg: string;
}

/**
 * Where the backup being restored comes from: a local file picked via the OS
 * dialog, or a file in the configured WebDAV server's backup directory (the
 * connection is read from the saved config by the Rust commands, so nothing
 * sensitive passes through here).
 */
export type ImportSource =
  | { kind: "file"; path: string }
  | { kind: "webdav"; name: string };

const JSON_FILTER = [{ name: "JSON", extensions: ["json"] }];
const CONFIRM_COUNTDOWN_SECONDS = 5;

export interface UseDataImportOptions {
  /** i18n message keys (resolved by the caller via t()) for status strings. */
  messages: {
    success: string;
    error: (message: string) => string;
  };
  /** Invoked after a successful restore. Default: hot-reload config and
   *  set a success status. The onboarding page overrides this to reload config
   *  and advance to the post-restore summary step. */
  onSuccess?: () => void | Promise<void>;
}

export function useDataImport(opts: UseDataImportOptions) {
  const importSource = ref<ImportSource | null>(null);
  const importPassword = ref("");
  const importShowPw = ref(false);
  const importConfirming = ref(false);
  const importCountdown = ref(CONFIRM_COUNTDOWN_SECONDS);
  const importStatus = ref<ImportStatus>({ kind: "idle", msg: "" });
  const importBusy = ref(false);

  // Analyze step: after entering a password the user clicks Analyze, which
  // inspects the bundle. The returned preview drives the category selector.
  const importPreview = ref<CategoryPreview[]>([]);
  const importSelected = ref<Set<string>>(new Set());
  const importAnalyzed = ref(false);
  const importAnalyzing = ref(false);

  const importTimer = useIntervalFn(() => {
    if (importCountdown.value > 0) importCountdown.value--;
    else importTimer.pause();
  }, 1000, { immediate: false });

  /** Display path of the current source (local path or remote file name);
   *  null when no source is chosen. */
  const importPath = computed(() => {
    const s = importSource.value;
    if (!s) return null;
    return s.kind === "file" ? s.path : s.name;
  });

  const importSourceKind = computed(() => importSource.value?.kind ?? null);

  const importFileName = computed(() => {
    const s = importSource.value;
    if (!s) return "";
    if (s.kind === "webdav") return s.name;
    const parts = s.path.replace(/\\/g, "/").split("/");
    return parts[parts.length - 1] || s.path;
  });

  // Analyze requires a source + a non-empty password (the bundle is password-
  // encrypted, so even inspection needs the password to unwrap the Master Key).
  const importCanAnalyze = computed(
    () => !!importSource.value && importPassword.value.length > 0 && !importAnalyzing.value,
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

  /** Common reset after a new source is chosen: a different backup means the
   *  password, analysis and countdown from the previous one no longer apply. */
  function onSourceChosen() {
    importPassword.value = "";
    clearAnalyzed();
    stopCountdown();
    importStatus.value = { kind: "idle", msg: "" };
  }

  async function selectImportFile() {
    const selected = await open({ multiple: false, filters: JSON_FILTER });
    const path = typeof selected === "string" ? selected : null;
    // User dismissed the file dialog — that's their own action, nothing to
    // announce. Leave any existing status untouched so a stale error from a
    // prior attempt isn't wiped either.
    if (!path) return;
    importSource.value = { kind: "file", path };
    onSourceChosen();
  }

  /** Choose a backup file on the configured WebDAV server (the caller lists
   *  the remote directory via `webdav_list_files` and passes the picked name). */
  function selectWebdavFile(name: string) {
    if (!name) return;
    importSource.value = { kind: "webdav", name };
    onSourceChosen();
  }

  function resetImport() {
    importSource.value = null;
    importPassword.value = "";
    importShowPw.value = false;
    clearAnalyzed();
    stopCountdown();
  }

  async function analyzeImport() {
    const source = importSource.value;
    if (!importCanAnalyze.value || !source) return;
    importAnalyzing.value = true;
    importStatus.value = { kind: "idle", msg: "" };
    try {
      const preview = source.kind === "file"
        ? await invoke<BundlePreview>("inspect_bundle", {
            path: source.path,
            password: importPassword.value,
          })
        : await invoke<BundlePreview>("webdav_inspect_file", {
            name: source.name,
            password: importPassword.value,
          });
      importPreview.value = preview.categories;
      // Default to restoring all available categories except settings, WebDAV
      // server settings and history (those are machine-specific and rarely
      // need restoring).
      importSelected.value = new Set(
        preview.categories
          .map((c) => c.id)
          .filter((id) => id !== "settings" && id !== "history" && id !== "webdav"),
      );
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
    const source = importSource.value;
    if (importCountdown.value > 0 || !source || !importAnalyzed.value) return;
    importBusy.value = true;
    try {
      if (source.kind === "file") {
        await invoke("import_data", {
          path: source.path,
          password: importPassword.value,
          categories: [...importSelected.value],
        });
      } else {
        await invoke("webdav_restore_file", {
          name: source.name,
          password: importPassword.value,
          categories: [...importSelected.value],
        });
      }
      // Default hot-reload: re-hydrate config + encrypted stores so the UI
      // reflects the restored data without a restart. Callers that override
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
    importSourceKind,
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
    selectWebdavFile,
    analyzeImport,
    requestImport,
    confirmImport,
    stopCountdown,
    resetImport,
  };
}
