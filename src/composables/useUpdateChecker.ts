/**
 * Shared update-checker singleton.
 *
 * Lifted out of Settings.vue so the update state has ONE owner across the
 * whole app lifetime: Settings.vue, FloatingInput.vue (the new-version red
 * dot on its settings button), and the launch-time check in main.ts all read
 * from the same module-level refs. The first view to mount no longer "wins"
 * the check, and FloatingInput no longer has to wait for the user to visit
 * Settings before it can show the dot.
 */
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import i18n from "../i18n";

// idle | checking | up-to-date | has-update | preparing | downloading | installing | restarting | error
export const updateStatus = ref("idle");
export const updateVersion = ref("");
export const downloaded = ref(0);
export const contentLength = ref(0);
export const updateError = ref("");

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Schedule a temporary status, then reset to idle after `ms`. Used by the
 *  silent-check error paths. */
function scheduleUpdateReset(ms: number) {
  setTimeout(() => {
    updateStatus.value = "idle";
    updateError.value = "";
  }, ms);
}

export async function checkForUpdate(silent = false) {
  if (!isTauri) return;
  updateStatus.value = "checking";
  updateError.value = "";
  // Sandbox: short-circuit to a permanent "has-update" state so the entire
  // update UI flow (badge, banner, install button) is exercised without a
  // real network round-trip. installUpdate() blocks the actual install.
  try {
    const sandbox = await invoke<boolean>("is_sandbox");
    if (sandbox) {
      updateVersion.value = "0.0.0";
      updateStatus.value = "has-update";
      return;
    }
  } catch { /* ignore — fall through to real check */ }
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const proxy = await invoke<string | null>("get_proxy_url");
    const update = await check(proxy ? { proxy } : {});
    if (!update) {
      if (silent) {
        updateStatus.value = "idle";
      } else {
        updateStatus.value = "up-to-date";
        scheduleUpdateReset(2000);
      }
      return;
    }
    updateVersion.value = update.version;
    updateStatus.value = "has-update";
  } catch (e) {
    if (!silent) {
      updateStatus.value = "error";
      updateError.value = e instanceof Error ? e.message : String(e);
      scheduleUpdateReset(3000);
    } else {
      updateStatus.value = "idle";
    }
  }
}

export async function installUpdate() {
  if (!isTauri) return;
  // Sandbox: don't actually download/install — the "has-update" state was
  // faked by checkForUpdate. Surface a brief error so the user knows.
  try {
    const sandbox = await invoke<boolean>("is_sandbox");
    if (sandbox) {
      updateStatus.value = "error";
      updateError.value = i18n.global.t("about.sandboxUpdateBlocked");
      scheduleUpdateReset(3000);
      return;
    }
  } catch { /* ignore — fall through */ }
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const { relaunch } = await import("@tauri-apps/plugin-process");
    const proxy = await invoke<string | null>("get_proxy_url");
    const update = await check(proxy ? { proxy } : {});
    if (!update) return;
    updateStatus.value = "preparing";
    downloaded.value = 0;
    contentLength.value = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength.value = event.data.contentLength || 0;
          updateStatus.value = "downloading";
          break;
        case "Progress":
          downloaded.value += event.data.chunkLength;
          break;
        case "Finished":
          updateStatus.value = "installing";
          break;
      }
    });
    updateStatus.value = "restarting";
    await relaunch();
  } catch (e) {
    updateStatus.value = "error";
    updateError.value = e instanceof Error ? e.message : String(e);
    scheduleUpdateReset(3000);
  }
}

// Statuses that disable interaction with the update button (busy / in-flight).
const UPDATE_BUSY_STATUSES = ["checking", "preparing", "downloading", "installing", "restarting"];

/** Human label for the update button, keyed off the current status. Reads
 *  i18n.global so it works outside of a component setup (this is a module
 *  singleton). */
export const updateLabel = computed(() => {
  switch (updateStatus.value) {
    case "checking": return i18n.global.t("about.checking");
    case "up-to-date": return i18n.global.t("about.upToDate");
    case "has-update": return i18n.global.t("about.install");
    case "preparing": return i18n.global.t("about.preparing");
    case "downloading": return contentLength.value > 0 ? "" : i18n.global.t("about.downloading");
    case "installing": return i18n.global.t("about.installing");
    case "restarting": return i18n.global.t("about.restarting");
    case "error": return updateError.value || i18n.global.t("about.checkFailed");
    default: return i18n.global.t("about.checkUpdate");
  }
});

export const updateDisabled = computed(() => UPDATE_BUSY_STATUSES.includes(updateStatus.value));

export const updateProgressPct = computed(() =>
  updateStatus.value === "downloading" && contentLength.value > 0
    ? Math.round(downloaded.value / contentLength.value * 100)
    : null,
);

export function handleUpdateClick() {
  if (updateStatus.value === "has-update") installUpdate();
  else if (["idle", "up-to-date", "error"].includes(updateStatus.value)) checkForUpdate(false);
}

export const isUpdateTauri = isTauri;

