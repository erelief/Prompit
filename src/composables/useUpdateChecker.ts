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
import type { Update } from "@tauri-apps/plugin-updater";
import i18n from "../i18n";
import { proxyFetch } from "../services/proxy";
import { pickLocalizedReleaseNotes } from "../stores/config";

// idle | checking | up-to-date | has-update | preparing | downloading | installing | restarting | error
export const updateStatus = ref("idle");
export const updateVersion = ref("");
// Release-notes text for the available update. Populated from Update.body
// (the release workflow injects the release body into the manifest's `notes`),
// or — for releases published before that — the GitHub Releases API `body`.
// This holds the FULL bilingual body (English\n---\nChinese); the popup binds
// to `displayNotes` which shows only the current UI locale's block.
export const updateNotes = ref("");
// True while a release-notes API fetch is in flight (so the popup can show a
// spinner instead of an empty/error state).
export const updateNotesLoading = ref(false);
// True when a release-notes fetch failed (network / 404 / parse). The popup
// shows a single "couldn't load" message in every case — the cause isn't
// actionable for the user.
export const updateNotesFailed = ref(false);
// The release-notes block for the CURRENT UI locale (English or Chinese),
// derived from the bilingual `updateNotes`. Re-computes when the notes load or
// when the user switches app language. Bind the popup to this, not updateNotes.
export const displayNotes = computed(() => pickLocalizedReleaseNotes(updateNotes.value));
export const downloaded = ref(0);
export const contentLength = ref(0);
export const updateError = ref("");

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// The Update object from the last successful check, kept so installUpdate()
// can download it directly instead of re-running check() — a second GitHub
// round-trip at click time is one more chance to fail on a flaky connection,
// and the user was already told which version they'll get. Not reactive; the
// refs above carry everything the UI needs.
let pendingUpdate: Update | null = null;

// GitHub Releases API for the latest published release. Its `body` field is the
// full release notes (markdown), which is exactly what the popup shows.
const RELEASES_API_URL = "https://api.github.com/repos/erelief/Prompit/releases/latest";

/** Fetch the latest release's `body` (release notes) from the GitHub Releases
 *  API into `updateNotes`. Used in sandbox (no real Update object) and as the
 *  prod fallback when Update.body is empty. On success with `adoptTag`, copies
 *  the release's tag_name into `updateVersion` — only the sandbox caller needs
 *  this (it fakes "0.0.0"; prod already has the real version). */
async function fetchReleaseNotes(adoptTag = false) {
  updateNotesFailed.value = false;
  updateNotesLoading.value = true;
  try {
    const res = await proxyFetch(RELEASES_API_URL, {
      method: "GET",
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) { updateNotesFailed.value = true; return; }
    const release = res.body ? JSON.parse(res.body) : null;
    const body = typeof release?.body === "string" ? release.body : "";
    if (!body) { updateNotesFailed.value = true; return; }
    updateNotes.value = body;
    if (adoptTag && typeof release?.tag_name === "string") {
      updateVersion.value = release.tag_name.replace(/^v/, "");
    }
  } catch {
    // proxyFetch rejects on transport-level failures (DNS, timeout, …).
    updateNotesFailed.value = true;
  } finally {
    updateNotesLoading.value = false;
  }
}

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
      updateNotes.value = "";
      updateStatus.value = "has-update";
      // Populate the release-notes popup with the latest published release
      // body from the GitHub API (the fake-update state has no Update object).
      // adoptTag: copy the API's tag_name so the popup header shows a real
      // version instead of the faked "0.0.0". Non-blocking.
      void fetchReleaseNotes(true);
      return;
    }
  } catch { /* ignore — fall through to real check */ }
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const proxy = await invoke<string | null>("get_proxy_url");
    const update = await check(proxy ? { proxy } : {});
    if (!update) {
      pendingUpdate = null;
      if (silent) {
        updateStatus.value = "idle";
      } else {
        updateStatus.value = "up-to-date";
        scheduleUpdateReset(2000);
      }
      return;
    }
    pendingUpdate = update;
    updateVersion.value = update.version;
    updateNotes.value = typeof update.body === "string" ? update.body : "";
    updateStatus.value = "has-update";
    // Update.body carries the notes for releases cut after the workflow
    // started embedding them; fall back to the GitHub Releases API `body`
    // for older releases whose manifest `notes` is empty.
    if (!updateNotes.value) {
      void fetchReleaseNotes();
    }
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
    // Reuse the Update from the last check rather than hitting GitHub again.
    // Fall back to a fresh check only when this webview never ran one (e.g.
    // the startup check ran in a different window).
    const update = pendingUpdate ?? (await check(proxy ? { proxy } : {}));
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

