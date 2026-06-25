<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter, useRoute } from "vue-router";
import {
  appConfig,
  personaStore,
  sparkleStore,
  flushConfigSave,
  savePersonas as persistPersonas,
  saveSparkles as persistSparkles,
  getOrderedLanguages,
  loadProviderPresets,
  dictStore,
  refreshDictStatus,
  clearAllHistory,
  MODES,
} from "../stores/config";
import { burstParticles } from "../utils/burstParticles";
import { keyCodeToToken, shortcutsEqual } from "../utils/shortcut";
import type { ProviderConfig, ProviderPreset, PresetVariantEndpoint, PresetVariantRegion } from "../stores/config";
import {
  getProviderIcon,
  getProviderSeries,
  resolvePreset,
  presetBelongsToFamily,
  defaultSelection,
  endpointsOf,
  defaultEndpoint,
  regionLabel,
  endpointLabel,
  variantRegionLabelKey,
  variantEndpointLabelKey,
} from "../stores/config";
import ProviderIcon from "../components/icons/providers/ProviderIcon.vue";
import ModelCapabilityIcon from "../components/ModelCapabilityIcon.vue";
import { getTheme, setTheme } from "../composables/useTheme";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { testProviderConnection, fetchProviderModels, optimizePrompt } from "../services/llm-client";
import type { FetchModelEntry } from "../services/llm-client";
import { SEARCH_PRESETS, presetMeta, testWebEngine } from "../services/websearch";
import type { WebEngineConfig } from "../stores/config";
import type { ModelInputCapabilities } from "../stores/config";
import { BUILTIN_LANGUAGES, getLangName } from "../constants/languages";
import draggable from "vuedraggable";
import EditableCardList from "../components/EditableCardList.vue";
import {
  ArrowLeft,
  Languages,
  UserCircle,
  Settings2,
  Plus,
  Trash2,
  Check,
  Eye,
  EyeOff,
  Link2,
  Loader2,
  Wand2,
  RefreshCw,
  ChevronDown,
  Pencil,
  Cpu,
  CircleDot,
  X,
  BookText,
  GripVertical,
  RotateCcw,
  CloudDownload,
  Sun,
  Moon,
  SunMoon,
  Info,
  ToggleLeft,
  ToggleRight,
  Droplet,
  Database,
  Monitor,
  History,
  Sparkles,
  Cloudy,
  Keyboard,
  SlidersHorizontal,
  Globe,
} from "@lucide/vue";

declare const __APP_VERSION__: string;
const appVersion = __APP_VERSION__;

const { t } = useI18n();

type TabKey = "general" | string;

const router = useRouter();
const route = useRoute();
const { growAbove } = useSettingsWindow();
const activeTab = ref<TabKey>("general");
const testingProvider = ref<number | null>(null);
const optimizingIndex = ref<number | null>(null);
const promptUndoStack = new Map<number, string>();
const summarizingIndex = ref<number | null>(null);
const descUndoStack = new Map<number, string>();

interface ProviderEditState {
  keyVisible: boolean;
  fetching: boolean;
  fetched: FetchModelEntry[];
  status: string;
  ok: boolean;          // true = last test/fetch succeeded (drives status-pill color)
  manualInput: string;
}
const editStates = ref<Map<number, ProviderEditState>>(new Map());

function getEditState(index: number): ProviderEditState {
  let s = editStates.value.get(index);
  if (!s) {
    s = reactive({ keyVisible: false, fetching: false, fetched: [], status: "", ok: false, manualInput: "" });
    editStates.value.set(index, s);
    editStates.value = new Map(editStates.value);
  }
  return s;
}

function clearEditState(index: number) {
  editStates.value.delete(index);
}

const addingModelProvider = ref<number | null>(null);
const showModelSelector = ref(false);
const translationShowLangSelector = ref(false);
const showPresetMenu = ref(false);
const presetMenuPos = ref({ top: 0, left: 0 });
const presetMenuIndex = ref<number | null>(null);
const providerPresets = ref<ProviderPreset[]>([]);
const selMenuPos = ref({ top: 0, left: 0 });
const translationLangMenuPos = ref({ top: 0, left: 0 });
const selBtnRef = ref<HTMLElement | null>(null);
const translationLangBtnRef = ref<HTMLElement | null>(null);

// ── App language switcher ──
const appLanguageOptions = [
  { value: "en", label: "English" },
  { value: "zh-CN", label: "简体中文" },
];

const showAppLangMenu = ref(false);
const showHistoryClearConfirm = ref(false);
const appLangMenuPos = ref({ top: 0, left: 0, width: 0 });
const appLangBtnRef = ref<HTMLElement | null>(null);

function toggleAppLangMenu() {
  showAppLangMenu.value = !showAppLangMenu.value;
  if (showAppLangMenu.value && appLangBtnRef.value) {
    const r = appLangBtnRef.value.getBoundingClientRect();
    appLangMenuPos.value = { top: r.bottom + 5, left: r.left, width: r.width };
  }
}

function selectAppLang(lang: string) {
  appConfig.app_lang = lang;
  showAppLangMenu.value = false;
}

const currentAppLangLabel = computed(() => {
  return appLanguageOptions.find(o => o.value === appConfig.app_lang)?.label || "English";
});

// ── Auto-update ──
// idle | checking | up-to-date | has-update | preparing | downloading | installing | restarting | error
const updateStatus = ref("idle");
const updateVersion = ref("");
const downloaded = ref(0);
const contentLength = ref(0);
const updateError = ref("");
const autoUpdate = ref(localStorage.getItem("app-auto-update") !== "false");
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function toggleAutoUpdate(e: MouseEvent) {
  const turning = !autoUpdate.value;
  autoUpdate.value = turning;
  localStorage.setItem("app-auto-update", String(turning));
  if (turning) burstParticles(e.currentTarget as HTMLElement);
}

function toggleShortcutHint(e: MouseEvent) {
  const turning = !appConfig.show_startup_reminder;
  appConfig.show_startup_reminder = turning;
  if (turning) burstParticles(e.currentTarget as HTMLElement);
}

// ── Launch on startup (OS autostart registration) ──
async function toggleLaunchOnStartup(e: MouseEvent) {
  if (!isTauri) return;
  const turning = !appConfig.launch_on_startup;
  // Optimistically flip; revert on failure so the toggle reflects truth.
  appConfig.launch_on_startup = turning;
  try {
    const { enable, disable } = await import("@tauri-apps/plugin-autostart");
    if (turning) {
      await enable();
      burstParticles(e.currentTarget as HTMLElement);
    } else {
      await disable();
    }
  } catch (err) {
    console.error("Toggle launch-on-startup failed:", err);
    appConfig.launch_on_startup = !turning;
  }
}

// ── Shortcut recorder ──
const shortcutRecording = ref(false);
const shortcutError = ref("");
const shortcutRecBtn = ref<HTMLButtonElement | null>(null);
// Auto-dismiss timer for the validation error so it fades after ~1.8s without
// requiring the user to click anything.
let shortcutErrorTimer: ReturnType<typeof setTimeout> | null = null;

/** Show a validation error that auto-clears itself after ~1.8s. Exits the
 *  recording state so the error text is actually rendered (the template's
 *  `v-if="recording"` branch would otherwise keep the button stuck on the
 *  "Press a shortcut…" label and hide the message). */
function showShortcutError(msg: string) {
  shortcutRecording.value = false;
  if (isTauri) invoke("finish_record_shortcut").catch(() => { /* best-effort */ });
  if (shortcutErrorTimer) clearTimeout(shortcutErrorTimer);
  shortcutError.value = msg;
  shortcutErrorTimer = setTimeout(() => {
    shortcutError.value = "";
    shortcutErrorTimer = null;
  }, 1800);
}

/** Immediately clear the validation error and its auto-hide timer. */
function clearShortcutError() {
  if (shortcutErrorTimer) { clearTimeout(shortcutErrorTimer); shortcutErrorTimer = null; }
  shortcutError.value = "";
}

// Split the current shortcut string into display tokens, e.g. "Ctrl+Shift+P" → ["Ctrl","Shift","P"]
const shortcutTokens = computed(() => appConfig.shortcut.split("+").map((s) => s.trim()).filter(Boolean));

function startShortcutRecord() {
  if (!isTauri) return;
  clearShortcutError();
  shortcutRecording.value = true;
  // Release the OS-level hotkey so raw key presses reach the webview while recording.
  invoke("start_record_shortcut").catch(() => { /* best-effort */ });
  nextTick(() => shortcutRecBtn.value?.focus());
}

async function cancelShortcutRecord() {
  if (!shortcutRecording.value) return;
  shortcutRecording.value = false;
  clearShortcutError();
  // Restore the saved binding since the user did not pick a new one.
  if (isTauri) await invoke("finish_record_shortcut").catch(() => {});
}

async function onShortcutKeydown(e: KeyboardEvent) {
  if (!shortcutRecording.value) return;
  e.preventDefault();
  e.stopPropagation();

  // Escape cancels without change; Backspace restores default
  if (e.code === "Escape") { await cancelShortcutRecord(); return; }
  if (e.code === "Backspace") {
    shortcutRecording.value = false;
    await applyShortcut("Alt+Y");
    return;
  }

  const token = keyCodeToToken(e.code);
  // Modifier-only presses (Alt/Ctrl/Shift/Cmd alone) carry no main key token:
  // stay quiet and wait for the actual key instead of flashing an error.
  if (!token) return;
  // A real key was pressed but no modifier is held → invalid. Auto-clears so
  // the user can immediately retry without dismissing anything by hand.
  if (!e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
    showShortcutError(t("settings.shortcutInvalid"));
    return;
  }

  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Cmd");
  const candidate = [...mods, token].join("+");
  // Reject if it collides with the mode-switch shortcut (the two must stay
  // distinct so neither binding silently shadows the other).
  if (shortcutsEqual(candidate, appConfig.mode_shortcut)) {
    showShortcutError(t("settings.shortcutConflict"));
    return;
  }
  shortcutRecording.value = false;
  await applyShortcut(candidate);
}

async function applyShortcut(shortcut: string) {
  clearShortcutError();
  if (!isTauri) return;
  try {
    await invoke("update_shortcut", { shortcut });
    appConfig.shortcut = shortcut; // existing watcher persists config.json
    if (shortcutRecBtn.value) burstParticles(shortcutRecBtn.value);
  } catch {
    showShortcutError(t("settings.shortcutConflict"));
    // update_shortcut already rolled back to the previous binding; nothing to restore.
  }
}

// Restore the wake-up shortcut to its factory default (Alt+Y). Same conflict
// guard as manual recording: refuses if the mode-switch shortcut currently
// holds that binding.
async function resetShortcut() {
  const def = "Alt+Y";
  if (shortcutsEqual(def, appConfig.shortcut)) return;
  if (shortcutsEqual(def, appConfig.mode_shortcut)) {
    showShortcutError(t("settings.shortcutConflict"));
    return;
  }
  shortcutRecording.value = false;
  await applyShortcut(def);
}

// ── Mode-switch shortcut recorder ──
// Unlike the wake shortcut, this is a webview-scoped binding (not an OS global
// hotkey), so there is no Rust re-registration: we just persist the token
// string and the FloatingInput listener reads it at keydown time.
const modeShortcutRecording = ref(false);
const modeShortcutError = ref("");
const modeShortcutRecBtn = ref<HTMLButtonElement | null>(null);
let modeShortcutErrorTimer: ReturnType<typeof setTimeout> | null = null;

function showModeShortcutError(msg: string) {
  modeShortcutRecording.value = false;
  if (modeShortcutErrorTimer) clearTimeout(modeShortcutErrorTimer);
  modeShortcutError.value = msg;
  modeShortcutErrorTimer = setTimeout(() => {
    modeShortcutError.value = "";
    modeShortcutErrorTimer = null;
  }, 1800);
}

function clearModeShortcutError() {
  if (modeShortcutErrorTimer) { clearTimeout(modeShortcutErrorTimer); modeShortcutErrorTimer = null; }
  modeShortcutError.value = "";
}

const modeShortcutTokens = computed(() =>
  appConfig.mode_shortcut.split("+").map((s) => s.trim()).filter(Boolean)
);

function startModeShortcutRecord() {
  clearModeShortcutError();
  modeShortcutRecording.value = true;
  nextTick(() => modeShortcutRecBtn.value?.focus());
}

function cancelModeShortcutRecord() {
  modeShortcutRecording.value = false;
  clearModeShortcutError();
}

async function onModeShortcutKeydown(e: KeyboardEvent) {
  if (!modeShortcutRecording.value) return;
  e.preventDefault();
  e.stopPropagation();

  if (e.code === "Escape") { cancelModeShortcutRecord(); return; }
  if (e.code === "Backspace") {
    modeShortcutRecording.value = false;
    appConfig.mode_shortcut = "Alt+M";
    return;
  }

  const token = keyCodeToToken(e.code);
  // Modifier-only presses (Alt/Ctrl/Shift/Cmd alone) carry no main key token:
  // stay quiet and wait for the actual key instead of flashing an error.
  if (!token) return;
  // A real key was pressed but no modifier is held → invalid. Auto-clears so
  // the user can immediately retry without dismissing anything by hand.
  if (!e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
    showModeShortcutError(t("settings.shortcutInvalid"));
    return;
  }
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Cmd");
  const candidate = [...mods, token].join("+");
  // Reject if it collides with the wake-up shortcut.
  if (shortcutsEqual(candidate, appConfig.shortcut)) {
    showModeShortcutError(t("settings.shortcutConflict"));
    return;
  }
  modeShortcutRecording.value = false;
  clearModeShortcutError();
  appConfig.mode_shortcut = candidate;
  if (modeShortcutRecBtn.value) burstParticles(modeShortcutRecBtn.value);
}

// Restore the mode-switch shortcut to its factory default (Alt+M). Refuses if
// the wake-up shortcut currently holds that binding.
function resetModeShortcut() {
  const def = "Alt+M";
  if (shortcutsEqual(def, appConfig.mode_shortcut)) return;
  if (shortcutsEqual(def, appConfig.shortcut)) {
    showModeShortcutError(t("settings.shortcutConflict"));
    return;
  }
  modeShortcutRecording.value = false;
  clearModeShortcutError();
  appConfig.mode_shortcut = def;
  if (modeShortcutRecBtn.value) burstParticles(modeShortcutRecBtn.value);
}

function toggleTranslationDict(e: MouseEvent) {
  const turning = !appConfig.user_dict_enabled;
  appConfig.user_dict_enabled = turning;
  if (turning) burstParticles(e.currentTarget as HTMLElement);
}

async function checkForUpdate(silent = false) {
  if (!isTauri) return;
  updateStatus.value = "checking";
  updateError.value = "";
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const proxy = await invoke<string | null>("get_proxy_url");
    const update = await check(proxy ? { proxy } : {});
    if (!update) {
      if (!silent) {
        updateStatus.value = "up-to-date";
        setTimeout(() => { updateStatus.value = "idle"; }, 2000);
      } else {
        updateStatus.value = "idle";
      }
      return;
    }
    updateVersion.value = update.version;
    updateStatus.value = "has-update";
  } catch (e) {
    if (!silent) {
      updateStatus.value = "error";
      updateError.value = e instanceof Error ? e.message : String(e);
      setTimeout(() => { updateStatus.value = "idle"; updateError.value = ""; }, 3000);
    } else {
      updateStatus.value = "idle";
    }
  }
}

async function installUpdate() {
  if (!isTauri) return;
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
    setTimeout(() => { updateStatus.value = "idle"; updateError.value = ""; }, 3000);
  }
}

function handleUpdateClick() {
  if (updateStatus.value === "has-update") installUpdate();
  else if (["idle", "up-to-date", "error"].includes(updateStatus.value)) checkForUpdate(false);
}

// ── Persona management ──
function validateProvider(p: ProviderConfig): string | null {
  const missing: string[] = [];
  if (!p.name.trim()) missing.push("Name");
  if (!p.api_key.trim()) missing.push("API Key");
  if (!p.base_url.trim()) missing.push("Base URL");
  if (p.models.length === 0) missing.push("at least one Model");
  return missing.length ? `Required: ${missing.join(", ")}` : null;
}

function validateTranslationPersona(p: { name: string; prompt: string }, index: number): string | null {
  const missing: string[] = [];
  if (!p.name.trim()) missing.push("Name");
  if (!p.prompt.trim()) missing.push("Prompt");
  if (missing.length) return `Required: ${missing.join(", ")}`;
  const dup = personaStore.personas.findIndex(
    (o, i) => i !== index && o.name.trim().toLowerCase() === p.name.trim().toLowerCase()
  );
  if (dup !== -1) return t("settings.duplicateName");
  return null;
}

function toggleTranslationPersona(index: number, e: MouseEvent) {
  const wasOn = personaStore.personas[index].enabled;
  for (const p of personaStore.personas) p.enabled = false;
  if (!wasOn) {
    personaStore.personas[index].enabled = true;
    burstParticles(e.currentTarget as HTMLElement);
  }
}

async function handleTranslationOptimizePrompt(item: { prompt: string }, index: number) {
  if (!item.prompt.trim() || optimizingIndex.value !== null) return;
  promptUndoStack.set(index, item.prompt);
  optimizingIndex.value = index;
  try {
    item.prompt = await optimizePrompt(item.prompt);
  } catch (err) {
    console.error("Optimize failed:", err);
    promptUndoStack.delete(index);
  } finally {
    optimizingIndex.value = null;
  }
}

function handleTextareaKeydown(e: KeyboardEvent, item: { prompt: string }, index: number) {
  const isMod = e.ctrlKey || e.metaKey;
  if (isMod && e.key === "z" && !e.shiftKey && promptUndoStack.has(index)) {
    e.preventDefault();
    item.prompt = promptUndoStack.get(index)!;
    promptUndoStack.delete(index);
  }
}

function validateSparkle(s: { name: string; prompt: string }, index: number): string | null {
  const missing: string[] = [];
  if (!s.name.trim()) missing.push("Name");
  if (!s.prompt.trim()) missing.push("Prompt");
  if (missing.length) return `Required: ${missing.join(", ")}`;
  const dup = sparkleStore.sparkles.findIndex(
    (o, i) => i !== index && o.name.trim().toLowerCase() === s.name.trim().toLowerCase()
  );
  if (dup !== -1) return t("settings.duplicateName");
  return null;
}

function toggleSparkle(index: number, e: MouseEvent) {
  const wasOn = sparkleStore.sparkles[index].enabled;
  if (wasOn && sparkleStore.sparkles.length <= 1) return;
  for (const s of sparkleStore.sparkles) s.enabled = false;
  if (!wasOn) {
    sparkleStore.sparkles[index].enabled = true;
    burstParticles(e.currentTarget as HTMLElement);
  }
  persistSparkles();
}

// ── Web search engine management ──
interface WebEngineEditState {
  testing: boolean;
  status: string;        // "" | success text | error text
  ok: boolean;           // true = last test succeeded (drives status-pill color)
}
const webEngineEditStates = ref<Map<number, WebEngineEditState>>(new Map());
const webEngineShowKey = ref<Set<number>>(new Set());

function getWebEngineEditState(index: number): WebEngineEditState {
  let s = webEngineEditStates.value.get(index);
  if (!s) {
    s = { testing: false, status: "", ok: false };
    webEngineEditStates.value.set(index, s);
    webEngineEditStates.value = new Map(webEngineEditStates.value);
  }
  return s;
}

function onWebEngineAdd(draft: WebEngineConfig) {
  Object.assign(draft, {
    preset: SEARCH_PRESETS[0].id,
    api_key: "",
    enabled: false,
    custom_name: undefined,
  });
}

function onWebEngineConfirm({ index }: { index: number }) {
  // Draft → real index migration (mirrors provider confirm handling)
  const draftState = webEngineEditStates.value.get(-1);
  if (draftState) {
    webEngineEditStates.value.delete(-1);
    webEngineEditStates.value.set(index, draftState);
    webEngineEditStates.value = new Map(webEngineEditStates.value);
  }
  // Default the active index to the first confirmed engine so it can be used
  // once the user toggles it on. Stays -1 (anonymous fallback) until enabled.
  if (appConfig.web_search_active_index < 0) {
    appConfig.web_search_active_index = 0;
  }
  flushConfigSave();
}

function onWebEngineCancel() {
  webEngineEditStates.value.delete(-1);
  webEngineEditStates.value = new Map(webEngineEditStates.value);
}

function onWebEngineRemove({ index, indexMap }: { index: number; indexMap: Map<number, number> }) {
  webEngineEditStates.value.delete(index);
  const re = new Map<number, WebEngineEditState>();
  for (const [k, v] of webEngineEditStates.value) {
    const m = indexMap.get(k);
    if (m !== undefined) re.set(m, v);
  }
  webEngineEditStates.value = re;
  // Re-point active index; clamp to range, fall back to anonymous if none enabled
  const anyEnabled = appConfig.web_engines.some((e) => e.enabled);
  if (!anyEnabled) {
    appConfig.web_search_active_index = -1;
  } else if (appConfig.web_search_active_index >= appConfig.web_engines.length) {
    appConfig.web_search_active_index = appConfig.web_engines.findIndex((e) => e.enabled);
  }
  flushConfigSave();
}

function validateWebEngine(eng: WebEngineConfig): string | null {
  const meta = presetMeta(eng.preset);
  if (meta.keyRequired && !eng.api_key.trim()) {
    return t("settings.apiKeyRequired");
  }
  return null;
}

/** Exclusive toggle: only one engine may be enabled at a time. Mirrors
 *  toggleTranslationPersona. Key-empty engines can't be enabled (validated). */
function toggleWebEngineExclusive(index: number, e: MouseEvent) {
  const eng = appConfig.web_engines[index];
  const meta = presetMeta(eng.preset);
  if (meta.keyRequired && !eng.api_key) return; // safety: validation should have blocked this
  const wasOn = eng.enabled;
  for (const w of appConfig.web_engines) w.enabled = false;
  if (!wasOn) {
    eng.enabled = true;
    appConfig.web_search_active_index = index;
    burstParticles(e.currentTarget as HTMLElement);
  } else {
    appConfig.web_search_active_index = -1; // anonymous fallback
  }
  flushConfigSave();
}

function toggleWebEngineKeyVisible(index: number) {
  const s = new Set(webEngineShowKey.value);
  if (s.has(index)) s.delete(index);
  else s.add(index);
  webEngineShowKey.value = s;
}

async function testWebEngineConnection(eng: WebEngineConfig, index: number) {
  if (!eng.api_key.trim()) return;
  const s = getWebEngineEditState(index);
  s.testing = true;
  s.status = "";
  const r = await testWebEngine(eng.preset, eng.api_key);
  s.testing = false;
  s.ok = r.ok;
  if (r.ok) {
    s.status = t("settings.connected");
    setTimeout(() => { s.status = ""; }, 3000);
  } else {
    s.status = r.error || t("settings.failedToConnect");
    setTimeout(() => { s.status = ""; }, 4000);
  }
}

// ── Web search preset selector (mirrors the provider preset-mini-btn + sel-menu pattern) ──
const showWebPresetMenu = ref(false);
const webPresetMenuIndex = ref<number | null>(null);
const webPresetMenuPos = ref({ top: 0, left: 0 });

function toggleWebPresetMenu(e: MouseEvent, index: number) {
  if (showWebPresetMenu.value && webPresetMenuIndex.value === index) {
    showWebPresetMenu.value = false;
    webPresetMenuIndex.value = null;
    return;
  }
  webPresetMenuIndex.value = index;
  showWebPresetMenu.value = true;
  const btn = e.currentTarget as HTMLElement;
  const r = btn.getBoundingClientRect();
  const menuW = 220;
  let left = r.right - menuW;
  if (left + menuW > window.innerWidth - 8) left = window.innerWidth - 8 - menuW;
  if (left < 8) left = 8;
  webPresetMenuPos.value = { top: r.bottom + 5, left };
}

function applyWebPreset(item: WebEngineConfig, presetId: string) {
  item.preset = presetId;
  // Mirror providers: switching the preset overwrites the display name with
  // the preset's label (the provider-supplied name).
  item.custom_name = presetMeta(presetId).label;
  showWebPresetMenu.value = false;
  webPresetMenuIndex.value = null;
}

async function handleSparkleOptimizePrompt(item: { prompt: string }, index: number) {
  if (!item.prompt.trim() || optimizingIndex.value !== null) return;
  promptUndoStack.set(index, item.prompt);
  optimizingIndex.value = index;
  try {
    item.prompt = await optimizePrompt(item.prompt, "sparkle");
  } catch (err) {
    console.error("Organize failed:", err);
    promptUndoStack.delete(index);
  } finally {
    optimizingIndex.value = null;
  }
}

function handleDescKeydown(e: KeyboardEvent, item: { description: string }, index: number) {
  const isMod = e.ctrlKey || e.metaKey;
  if (isMod && e.key === "z" && !e.shiftKey && descUndoStack.has(index)) {
    e.preventDefault();
    item.description = descUndoStack.get(index)!;
    descUndoStack.delete(index);
  }
}

async function handleSparkleSummarize(item: { prompt: string; description: string }, index: number) {
  if (!item.prompt.trim() || summarizingIndex.value !== null) return;
  descUndoStack.set(index, item.description);
  summarizingIndex.value = index;
  try {
    item.description = await optimizePrompt(item.prompt, "summarize");
  } catch (err) {
    console.error("Summarize failed:", err);
    descUndoStack.delete(index);
  } finally {
    summarizingIndex.value = null;
  }
}

function toggleSelMenu() {
  if (allFlat.value.length === 0) return;
  translationShowLangSelector.value = false;
  showModelSelector.value = !showModelSelector.value;
  if (showModelSelector.value && selBtnRef.value) {
    const r = selBtnRef.value.getBoundingClientRect();
    selMenuPos.value = { top: r.bottom + 5, left: r.left };
  }
}

function toggleTranslationLangMenu() {
  showModelSelector.value = false;
  translationShowLangSelector.value = !translationShowLangSelector.value;
  if (translationShowLangSelector.value && translationLangBtnRef.value) {
    const r = translationLangBtnRef.value.getBoundingClientRect();
    translationLangMenuPos.value = { top: r.bottom + 5, left: r.left };
  }
}

function pickTranslationLang(lang: string) {
  appConfig.target_lang = lang;
  translationShowLangSelector.value = false;
}

function togglePresetMenu(e: MouseEvent, _item: ProviderConfig, index: number) {
  showModelSelector.value = false;
  translationShowLangSelector.value = false;
  if (showPresetMenu.value && presetMenuIndex.value === index) {
    showPresetMenu.value = false;
    presetMenuIndex.value = null;
    return;
  }
  presetMenuIndex.value = index;
  showPresetMenu.value = true;
  const btn = e.currentTarget as HTMLElement;
  const r = btn.getBoundingClientRect();
  const menuW = 220;
  // Right-align the menu to the button's right edge, clamped to the viewport
  let left = r.right - menuW;
  if (left + menuW > window.innerWidth - 8) left = window.innerWidth - 8 - menuW;
  if (left < 8) left = 8;
  presetMenuPos.value = { top: r.bottom + 5, left };
}

function applyPreset(item: ProviderConfig, preset: ProviderPreset) {
  if (preset.name === "Custom") {
    item.preset = undefined;
    item.base_url = "";
    item.api_format = undefined;
    showPresetMenu.value = false;
    presetMenuIndex.value = null;
    return;
  }
  // Variant family → land on its default region/endpoint selection.
  if (preset.variants) {
    const { endpoint } = defaultSelection(preset);
    applyEndpoint(item, preset, endpoint);
  } else {
    applyVariantFields(item, preset, undefined);
  }
  showPresetMenu.value = false;
  presetMenuIndex.value = null;
}

/** Write a specific endpoint (within a family) onto the form item. */
function applyEndpoint(item: ProviderConfig, family: ProviderPreset, endpoint?: PresetVariantEndpoint) {
  applyVariantFields(item, family, endpoint);
}

function applyVariantFields(
  item: ProviderConfig,
  preset: ProviderPreset,
  endpoint?: PresetVariantEndpoint,
) {
  item.preset = endpoint ? endpoint.provider_name : preset.name;
  item.base_url = endpoint ? endpoint.base_url : (preset.base_url ?? "");
  item.api_format = preset.api_format && Object.keys(preset.api_format).length > 0
    ? { ...preset.api_format }
    : undefined;
  item.name = endpoint ? endpoint.provider_name : (preset.provider_name ?? preset.name);
}

/** Switch the region; if the current endpoint key is unavailable in the new
 *  region, fall back to that region's default/first endpoint. */
function applyRegion(item: ProviderConfig, regionKey: string) {
  const family = resolvePreset(item.preset, providerPresets.value).preset;
  if (!family?.variants) return;
  const cur = resolvePreset(item.preset, providerPresets.value).endpoint;
  const eps = endpointsOf(family, regionKey);
  const next = eps.find(e => e.key === cur?.key) ?? defaultEndpoint(family, regionKey);
  if (next) applyEndpoint(item, family, next);
}

/** Switch the endpoint within the current region. */
function applyEndpointKey(item: ProviderConfig, endpointKey: string) {
  const { preset: family, region } = resolvePreset(item.preset, providerPresets.value);
  if (!family?.variants || !region) return;
  const ep = region.endpoints.find(e => e.key === endpointKey);
  if (ep) applyEndpoint(item, family, ep);
}

// ── Template helpers (variant selectors) ──
function variantFamily(item: ProviderConfig): ProviderPreset | undefined {
  return resolvePreset(item.preset, providerPresets.value).preset;
}
function variantRegions(item: ProviderConfig): PresetVariantRegion[] {
  return variantFamily(item)?.variants?.regions ?? [];
}
function variantEndpoints(item: ProviderConfig): PresetVariantEndpoint[] {
  const { preset: family, region } = resolvePreset(item.preset, providerPresets.value);
  return region ? region.endpoints : (family?.variants ? endpointsOf(family, family.variants.default_region ?? "") : []);
}
function currentRegionKey(item: ProviderConfig): string | undefined {
  return resolvePreset(item.preset, providerPresets.value).region?.key;
}
function currentEndpointKey(item: ProviderConfig): string | undefined {
  return resolvePreset(item.preset, providerPresets.value).endpoint?.key;
}
/** Resolve the "Get API key" link for the current preset (endpoint first). */
function presetApiKeyUrl(item: ProviderConfig): string | undefined {
  const { preset, endpoint } = resolvePreset(item.preset, providerPresets.value);
  return endpoint?.api_url || preset?.api_url;
}

// ── Language management ──
const translationNewLangInput = ref("");
const translationShowAddLang = ref(false);
const translationLangAddInputRef = ref<HTMLInputElement | null>(null);

watch(translationShowAddLang, (val) => {
  if (val) nextTick(() => translationLangAddInputRef.value?.focus());
});

interface TranslationLangItem {
  id: string;
  name: string;
  isCustom: boolean;
}

const translationLangItems = computed<TranslationLangItem[]>(() => {
  return getOrderedLanguages().map(name => ({
    id: name,
    name,
    isCustom: !BUILTIN_LANGUAGES.includes(name),
  }));
});

function onTranslationLangDragEnd() {
  appConfig.language_order = translationLangItems.value.map(item => item.name);
}

function onProviderDragEnd({ indexMap }: { indexMap: Map<number, number> }) {
  appConfig.translate_active_provider_index = indexMap.get(appConfig.translate_active_provider_index) ?? 0;

  const re = new Map<number, ProviderEditState>();
  for (const [k, v] of editStates.value) {
    const m = indexMap.get(k);
    if (m !== undefined) re.set(m, v);
  }
  editStates.value = re;

  if (testingProvider.value !== null) testingProvider.value = indexMap.get(testingProvider.value) ?? null;
  if (addingModelProvider.value !== null) addingModelProvider.value = indexMap.get(addingModelProvider.value) ?? null;
}

function onModelDragEnd(providerIndex: number, evt: { oldIndex: number; newIndex: number }) {
  const { oldIndex, newIndex } = evt;
  if (oldIndex === newIndex) return;
  // vuedraggable already mutated item.models; remap positional active-model indices so
  // the currently selected model stays selected after the reorder.
  const remap = (i: number): number => {
    if (i === oldIndex) return newIndex;
    if (oldIndex < newIndex) return (i > oldIndex && i <= newIndex) ? i - 1 : i;
    return (i >= newIndex && i < oldIndex) ? i + 1 : i;
  };
  if (appConfig.translate_active_provider_index === providerIndex) {
    appConfig.translate_active_model_index = remap(appConfig.translate_active_model_index);
  }
  if (appConfig.sparkle_active_provider_index === providerIndex) {
    appConfig.sparkle_active_model_index = remap(appConfig.sparkle_active_model_index);
  }
}

function deleteTranslationCustomLang(name: string) {
  appConfig.custom_languages = appConfig.custom_languages.filter(l => l !== name);
  appConfig.language_order = appConfig.language_order.filter(l => l !== name);
  if (appConfig.target_lang === name) {
    appConfig.target_lang = "English";
  }
}

function addTranslationCustomLang() {
  const name = translationNewLangInput.value.trim();
  if (!name) return;
  const allNames = getOrderedLanguages();
  if (allNames.some(l => l.toLowerCase() === name.toLowerCase())) {
    translationNewLangInput.value = "";
    return;
  }
  appConfig.custom_languages.push(name);
  appConfig.language_order = [...getOrderedLanguages(), name];
  translationNewLangInput.value = "";
  translationShowAddLang.value = false;
}

function restoreTranslationDefaultOrder() {
  appConfig.language_order = [];
}

function toggleKeyVisibility(index: number) {
  const s = getEditState(index);
  s.keyVisible = !s.keyVisible;
}

async function load() {
  // Config is loaded once at startup (main.ts) and shared as a single reactive
  // instance across all views — do not reload here, or disk (possibly stale)
  // would overwrite in-memory edits made in other views.
  refreshDictStatus();
}

// ── Auto-save (instant) ──
// Config auto-save is centralized in stores/config.ts (enabled at startup).
// Personas/sparkles still auto-persist here.
watch(
  () => JSON.stringify(personaStore.personas),
  () => { persistPersonas(); },
);

watch(
  () => JSON.stringify(sparkleStore.sparkles),
  () => { persistSparkles(); },
);

function onProviderAdd(draft: ProviderConfig) {
  Object.assign(draft, {
    name: "",
    api_key: "",
    base_url: "",
    models: [], temperature: null, max_tokens: null,
  });
}

function onProviderConfirm({ index }: { index: number }) {
  // Migrate edit state from draft index (-1) to the real index
  const draftState = editStates.value.get(-1);
  if (draftState) {
    editStates.value.delete(-1);
    editStates.value.set(index, draftState);
    editStates.value = new Map(editStates.value);
  }
  // Exit the fetch-picker state: selections have been committed, so the
  // pickable list should not linger when the card is reopened.
  const s = editStates.value.get(index);
  if (s) s.fetched = [];
  if (testingProvider.value === -1) testingProvider.value = index;
  flushConfigSave();
}

function onProviderCancel() {
  showPresetMenu.value = false;
  presetMenuIndex.value = null;
}

function onProviderRemove({ index, indexMap }: { index: number; indexMap: Map<number, number> }) {
  clearEditState(index);
  const re = new Map<number, ProviderEditState>();
  for (const [k, v] of editStates.value) {
    const m = indexMap.get(k);
    if (m !== undefined) re.set(m, v);
  }
  editStates.value = re;
  // Active provider/model indices are re-normalized centrally in
  // stores/config.ts (watch on provider/model counts), so no clamp here.
}

function removeModel(item: ProviderConfig, mIndex: number) {
  item.models.splice(mIndex, 1);
}

async function testConnection(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  testingProvider.value = index;
  const s = getEditState(index);
  const result = await testProviderConnection(provider);
  s.ok = result.ok;
  if (result.ok) {
    s.status = t("settings.connected");
    setTimeout(() => { s.status = ""; }, 3000);
  } else {
    s.status = result.error || t("settings.failedToConnect");
    setTimeout(() => { s.status = ""; }, 4000);
  }
  testingProvider.value = null;
}

async function fetchModels(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  const s = getEditState(index);
  s.fetching = true;
  const result = await fetchProviderModels(provider);
  if (result.ok && result.models) {
    // De-dup: drop any fetched entry that collides with a committed model id,
    // so no id appears twice in the merged pickable list.
    const committedIds = new Set(provider.models.map((m) => m.id));
    const fetchedUnique = result.models.filter((e) => !committedIds.has(e.id));
    // Merge: committed models (stays selected) + freshly fetched ones (selectable).
    s.fetched = [
      ...provider.models.map((m) => ({
        id: m.id,
        input_capabilities: m.input_capabilities || {},
      })),
      ...fetchedUnique,
    ];
  } else {
    s.ok = false;
    s.status = result.error || t("settings.fetchFailed");
    setTimeout(() => { s.status = ""; }, 5000);
  }
  s.fetching = false;
}

function toggleModel(item: ProviderConfig, entry: FetchModelEntry) {
  const idx = item.models.findIndex((m) => m.id === entry.id);
  if (idx >= 0) {
    item.models.splice(idx, 1);
  } else {
    item.models.push({
      id: entry.id,
      input_capabilities:
        Object.keys(entry.input_capabilities).length > 0
          ? entry.input_capabilities
          : undefined,
    });
  }
}

function addManualModel(item: ProviderConfig, index: number) {
  const s = getEditState(index);
  const id = s.manualInput.trim();
  if (!id) return;
  const existsInFetch = s.fetched.some((e) => e.id === id);
  const existsInCommitted = item.models.some((m) => m.id === id);
  // Only surface in the open fetch list if it's brand new (otherwise it's already shown there).
  if (!existsInFetch && !existsInCommitted && s.fetched.length > 0) {
    s.fetched = [...s.fetched, { id, input_capabilities: {} }];
  }
  // If the id already exists anywhere, just ensure it's selected (no-op if already selected).
  if (!existsInCommitted) {
    item.models.push({ id });
  }
  s.manualInput = "";
}

function getFetchedModels(pi: number): FetchModelEntry[] {
  return editStates.value.get(pi)?.fetched || [];
}

// ── Translation model selector ──

interface FlatEntry {
  pIndex: number;
  mIndex: number;
  id: string;
  providerName: string;
  icon: string;
  input_capabilities?: ModelInputCapabilities;
}

const allFlat = computed<FlatEntry[]>(() => {
  const out: FlatEntry[] = [];
  appConfig.providers.forEach((prov, pi) =>
    prov.models.forEach((m, mi) =>
      out.push({
        pIndex: pi,
        mIndex: mi,
        id: m.id,
        providerName: prov.name || `Provider ${pi + 1}`,
        icon: getProviderIcon(prov, providerPresets.value),
        input_capabilities: m.input_capabilities,
      })
    )
  );
  return out;
});

const translationActiveLabel = computed(() => {
  const { providers } = appConfig;
  const pi = appConfig.translate_active_provider_index;
  const mi = appConfig.translate_active_model_index;
  if (pi >= providers.length) return "None";
  const p = providers[pi];
  if (!p || mi >= p.models.length) return "None";
  return p.models[mi].id;
});

// Icon of the currently-selected translation model
const translationActiveIcon = computed(() => {
  const { providers } = appConfig;
  const pi = appConfig.translate_active_provider_index;
  if (pi >= providers.length) return "";
  const p = providers[pi];
  return p ? getProviderIcon(p, providerPresets.value) : "";
});

function pickTranslationModel(e: FlatEntry) {
  appConfig.translate_active_provider_index = e.pIndex;
  appConfig.translate_active_model_index = e.mIndex;
  showModelSelector.value = false;
  flushConfigSave();
}

function isTranslationModelActive(pIndex: number, mIndex: number): boolean {
  return pIndex === appConfig.translate_active_provider_index && mIndex === appConfig.translate_active_model_index;
}

function isSparkleModelActive(pIndex: number, mIndex: number): boolean {
  return pIndex === appConfig.sparkle_active_provider_index && mIndex === appConfig.sparkle_active_model_index;
}

const sparkleActiveLabel = computed(() => {
  const { providers } = appConfig;
  const pi = appConfig.sparkle_active_provider_index;
  const mi = appConfig.sparkle_active_model_index;
  if (pi >= providers.length) return "None";
  const p = providers[pi];
  if (!p || mi >= p.models.length) return "None";
  return p.models[mi].id;
});

// Icon of the currently-selected sparkle model
const sparkleActiveIcon = computed(() => {
  const { providers } = appConfig;
  const pi = appConfig.sparkle_active_provider_index;
  if (pi >= providers.length) return "";
  const p = providers[pi];
  return p ? getProviderIcon(p, providerPresets.value) : "";
});

function pickSparkleModel(e: FlatEntry) {
  appConfig.sparkle_active_provider_index = e.pIndex;
  appConfig.sparkle_active_model_index = e.mIndex;
  showModelSelector.value = false;
  flushConfigSave();
}

// ── Click outside panels ──

function onDocClick(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".sel-menu") && !t.closest(".sel-btn"))
    showModelSelector.value = false;
  if (!t.closest(".sel-menu") && !t.closest(".sel-btn"))
    showAppLangMenu.value = false;
  if (!t.closest(".lang-menu") && !t.closest(".lang-btn"))
    translationShowLangSelector.value = false;
  if (!t.closest(".preset-menu") && !t.closest(".preset-mini-btn")) {
    showPresetMenu.value = false;
    presetMenuIndex.value = null;
  }
  if (!t.closest(".web-preset-menu") && !t.closest(".web-preset-btn")) {
    showWebPresetMenu.value = false;
    webPresetMenuIndex.value = null;
  }
  if (!t.closest(".pickable"))
    addingModelProvider.value = null;
}

// ── Navigation ──

async function goBack() {
  router.push("/");
}

async function closeWindow() {
  await invoke("hide_main_window");
}

async function handleDrag(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("textarea, button, input, select, a, .ecl-card, .card-drag-handle, .sel-menu")) return;
  await getCurrentWindow().startDragging();
}

function openExternal(url: string) {
  if (isTauri) {
    import("@tauri-apps/plugin-shell").then(({ open }) => open(url));
  } else {
    window.open(url, "_blank");
  }
}

onMounted(async () => {
  if (route.query.tab === "translation") {
    activeTab.value = "translation";
  }
  if (route.query.scrollTo === "persona") {
    nextTick(() => {
      document.getElementById("persona-section")?.scrollIntoView({ behavior: "smooth" });
    });
  }
  document.addEventListener("mousedown", onDocClick);
  load();
  loadProviderPresets().then(p => { providerPresets.value = p; }).catch(console.error);
  if (autoUpdate.value) checkForUpdate(true);
  // Reconcile the launch-on-startup toggle with the actual OS entry, since the
  // user may have disabled it outside the app (e.g. Task Manager → Startup).
  if (isTauri) {
    import("@tauri-apps/plugin-autostart")
      .then(({ isEnabled }) => isEnabled())
      .then((enabled) => { appConfig.launch_on_startup = enabled; })
      .catch(() => { /* best-effort */ });
  }
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocClick);
});
</script>

<template>
  <div class="settings-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- ═══ Header ═══ -->
    <header class="settings-header">
      <button @click="goBack" class="back-btn" title="Back">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <h1 class="header-title">{{ t('common.settings') }}</h1>
      <button @click="closeWindow" class="close-btn" :title="t('settings.close')">
        <X :size="16" :stroke-width="1.8" />
      </button>
    </header>

    <!-- ═══ Tabs ═══ -->
    <nav class="tabs">
      <button
        class="tab"
        :class="{ on: activeTab === 'general' }"
        @click="activeTab = 'general'"
      >
        <Settings2 :size="13" :stroke-width="1.7" />
        {{ t('settings.general') }}
      </button>
      <button
        v-for="mode in MODES"
        :key="mode.id"
        class="tab"
        :class="{ on: activeTab === mode.settingTabKey }"
        @click="activeTab = mode.settingTabKey"
      >
        <component :is="mode.icon" :size="13" :stroke-width="1.7" />
        {{ t(mode.labelKey) }}
      </button>
    </nav>

    <!-- ═══ Body ═══ -->
    <main class="body">

      <!-- ─── General: Providers ─── -->
      <template v-if="activeTab === 'general'">
        <!-- Providers -->
        <EditableCardList
          :items="appConfig.providers"
          :title="t('settings.providers')"
          :icon="Cloudy"
          :empty-message="t('settings.noProvidersYet')"
          :empty-sub-message="t('settings.addOneToGetStarted')"
          :empty-icon="CircleDot"
          :validate="validateProvider"
          :allow-remove="appConfig.providers.length > 1"
          :builtin-drag-handle="false"
          @add="onProviderAdd"
          @confirm="onProviderConfirm"
          @cancel="onProviderCancel"
          @remove="onProviderRemove"
          @drag-end="onProviderDragEnd"
        >
          <template #collapsed="{ item }">
            <div class="prov-lhs">
              <span class="card-drag-handle prov-drag-logo" @click.stop>
                <ProviderIcon :icon="getProviderIcon(item, providerPresets)" :size="16" />
              </span>
              <div class="prov-accent" />
              <div class="prov-meta">
                <span class="prov-name" :class="{ dim: !item.name }">{{ item.name || t('settings.untitledProvider') }}</span>
                <span v-if="getProviderSeries(item, providerPresets)" class="prov-series-tag">{{ getProviderSeries(item, providerPresets) }}</span>
                <span class="prov-badge">{{ item.models.length }} {{ t('settings.model') }}</span>
              </div>
            </div>
          </template>

          <template #name-input="{ item, index }">
            <div class="name-row-wrap">
              <ProviderIcon :icon="getProviderIcon(item, providerPresets)" :size="16" />
              <input v-model="item.name" :placeholder="t('settings.providerName')" class="fi name-fi" @click.stop />
              <button
                class="preset-mini-btn"
                :class="{ active: item.preset }"
                @click.stop="togglePresetMenu($event, item, index)"
                :title="item.preset ? `${t('settings.preset')}: ${item.preset}` : t('settings.applyPreset')"
              >
                <CloudDownload :size="12" :stroke-width="1.8" />
              </button>
            </div>
          </template>

          <template #content="{ item, index }">
            <Teleport to="body">
              <Transition name="drop">
                <div v-if="showPresetMenu && presetMenuIndex === index" class="sel-menu preset-menu" :style="{ top: presetMenuPos.top + 'px', left: presetMenuPos.left + 'px', width: '220px' }">
                  <div class="sel-clip settings-scrollbar">
                    <button
                      v-for="p in providerPresets" :key="p.name"
                      class="sel-opt"
                      :class="{ hit: presetBelongsToFamily(item.preset, p) || (!item.preset && p.name === 'Custom') }"
                      @click="applyPreset(item, p)"
                    >
                      <div class="opt-left"><ProviderIcon :icon="p.icon" :size="14" />
                      <div class="opt-info">
                        <div class="opt-id-row">
                          <span class="opt-id">{{ p.name === 'Custom' ? t('onboarding.custom') : p.name }}</span>
                          <span v-if="p.model_series" class="opt-series-tag">{{ p.model_series }}</span>
                        </div>
                      </div></div>
                      <Check
                        v-if="presetBelongsToFamily(item.preset, p) || (!item.preset && p.name === 'Custom')"
                        :size="13" :stroke-width="2.5"
                      />
                    </button>
                    <div v-if="providerPresets.length === 0" class="preset-empty">
                      {{ t('settings.noPresetsFound') }}
                    </div>
                  </div>
                </div>
              </Transition>
            </Teleport>

            <!-- hint (compatibility note for non-preset / custom) -->
            <p v-if="!item.preset" class="preset-hint" @click.stop>
              {{ t('settings.openaiCompatHint') }}
            </p>

            <!-- variant selector (only for multi-variant family presets) -->
            <div
              v-if="variantFamily(item)?.variants"
              class="variant-block"
              @click.stop
            >
              <!-- Region row -->
              <div class="variant-row">
                <span class="variant-label">{{ t(variantRegionLabelKey()) }}</span>
                <div class="variant-btns">
                  <button
                    v-for="r in variantRegions(item)" :key="r.key"
                    class="variant-btn"
                    :class="{ active: currentRegionKey(item) === r.key }"
                    @click="applyRegion(item, r.key)"
                  >{{ regionLabel(r) }}</button>
                </div>
              </div>
              <!-- Endpoint row (options depend on the selected region) -->
              <div v-if="variantEndpoints(item).length > 0" class="variant-row">
                <span class="variant-label">{{ t(variantEndpointLabelKey()) }}</span>
                <div class="variant-btns">
                  <button
                    v-for="ep in variantEndpoints(item)" :key="ep.key"
                    class="variant-btn"
                    :class="{ active: currentEndpointKey(item) === ep.key }"
                    @click="applyEndpointKey(item, ep.key)"
                  >{{ endpointLabel(ep) }}</button>
                </div>
              </div>
            </div>

            <!-- get API key link (below variants, above fields) -->
            <p v-if="item.preset && presetApiKeyUrl(item)" class="preset-hint" @click.stop>
              <a :href="presetApiKeyUrl(item)" target="_blank" rel="noopener noreferrer" @click.prevent="openExternal(presetApiKeyUrl(item)!)" style="color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px;">
                {{ t('settings.getApiKeyAt', { name: item.preset }) }}
              </a>
            </p>

            <!-- fields -->
            <div class="fields">
              <div class="field">
                <label>{{ t('settings.apiKey') }}</label>
                <div class="key-wrap">
                  <input
                    v-model="item.api_key"
                    :type="editStates.get(index)?.keyVisible ? 'text' : 'password'"
                    class="fi key-fi" @click.stop
                  />
                  <button class="icon-btn-sm" @click.stop="toggleKeyVisibility(index)" :title="editStates.get(index)?.keyVisible ? 'Hide' : 'Show'">
                    <EyeOff v-if="editStates.get(index)?.keyVisible" :size="12" :stroke-width="1.9" />
                    <Eye v-else :size="12" :stroke-width="1.9" />
                  </button>
                  <button
                    class="icon-btn-sm linkish"
                    @click.stop="testConnection(item, index)"
                    :disabled="!item.api_key || testingProvider === index"
                    :title="t('settings.testConnection')"
                  >
                    <Loader2 v-if="testingProvider === index" :size="12" class="spin" :stroke-width="1.9" />
                    <Link2 v-else :size="12" :stroke-width="1.9" />
                  </button>
                </div>
                <Transition name="fade">
                  <span
                    v-if="editStates.get(index)?.status"
                    class="status-pill"
                    :class="{ ok: editStates.get(index)?.ok, err: !editStates.get(index)?.ok }"
                  >
                    <span class="status-dot" />
                    {{ editStates.get(index)?.status }}
                  </span>
                </Transition>
              </div>

              <div class="field">
                <label>{{ t('settings.baseUrl') }}</label>
                <input v-model="item.base_url" class="fi" placeholder="https://api.example.com/v1" @click.stop />
              </div>
            </div>

            <!-- pool -->
            <div class="pool-bar">
              <span class="pool-label">{{ t('settings.models') }} · {{ item.models.length }}</span>
              <div class="pool-actions">
                <button
                  class="pill-btn micro"
                  @click.stop="fetchModels(item, index)"
                  :disabled="!item.api_key || !item.base_url || editStates.get(index)?.fetching"
                >
                  <Loader2 v-if="editStates.get(index)?.fetching" :size="10" class="spin" :stroke-width="2" />
                  <RefreshCw v-else :size="10" :stroke-width="2" />
                  {{ editStates.get(index)?.fetching ? t('settings.fetching') : t('settings.fetch') }}
                </button>
              </div>
            </div>

            <!-- model tags (draggable) -->
            <draggable
              v-if="item.models.length > 0 && getFetchedModels(index).length === 0"
              :list="item.models"
              :item-key="(m: any) => m.id"
              handle=".model-drag-handle"
              :force-fallback="true"
              fallback-class="hidden-drag-ghost"
              ghost-class="model-ghost"
              chosen-class="model-chosen"
              class="tags"
              @end="onModelDragEnd(index, $event)"
            >
              <template #item="{ element }">
                <span class="tag">
                  <span class="model-drag-handle" @click.stop>
                    <GripVertical :size="9" :stroke-width="1.8" />
                  </span>
                  {{ element.id }}
                  <ModelCapabilityIcon :capabilities="element.input_capabilities" :size="10" />
                  <button class="tag-x" @click.stop="removeModel(item, item.models.indexOf(element))">
                    <Trash2 :size="9" :stroke-width="2" />
                  </button>
                </span>
              </template>
            </draggable>
            <div v-if="getFetchedModels(index).length > 0" class="tags pickable">
              <button
                v-for="entry in getFetchedModels(index)" :key="entry.id"
                class="tag"
                :class="{ selected: item.models.some((m: any) => m.id === entry.id) }"
                @click.stop="toggleModel(item, entry)"
              >
                <Check v-if="item.models.some((m: any) => m.id === entry.id)" :size="10" :stroke-width="2.5" />
                {{ entry.id }}
                <ModelCapabilityIcon :capabilities="entry.input_capabilities" :size="10" />
              </button>
              <!-- Manual model input (appended to fetched list) -->
              <div class="manual-model-tag" @click.stop>
                <input
                  :value="editStates.get(index)?.manualInput"
                  @input="getEditState(index).manualInput = ($event.target as HTMLInputElement).value"
                  @keydown.enter="addManualModel(item, index)"
                  class="manual-model-input"
                  :placeholder="t('onboarding.manualModelPlaceholder')"
                />
                <button
                  class="manual-model-add"
                  @click.stop="addManualModel(item, index)"
                  :disabled="!(editStates.get(index)?.manualInput || '').trim()"
                  :title="t('common.add')"
                >
                  <Plus :size="11" :stroke-width="2.2" />
                </button>
              </div>
            </div>
          </template>
          <template #disclaimer>
            <div class="api-disclaimer">
              <Info :size="11" :stroke-width="1.8" />
              <span>{{ t('settings.apiKeyDisclaimer') }}</span>
            </div>
          </template>
        </EditableCardList>

        <!-- Web Search -->
        <EditableCardList
          class="mt"
          :items="appConfig.web_engines"
          :title="t('settings.webSearch')"
          :icon="Globe"
          :empty-message="t('settings.webSearchEmpty')"
          :empty-sub-message="t('settings.addOneToGetStarted')"
          :empty-icon="Globe"
          :validate="validateWebEngine"
          :builtin-drag-handle="false"
          @add="onWebEngineAdd"
          @confirm="onWebEngineConfirm"
          @cancel="onWebEngineCancel"
          @remove="onWebEngineRemove"
        >
          <template #collapsed="{ item, index }">
            <div class="prov-lhs">
              <span class="card-drag-handle prov-drag-logo" @click.stop>
                <component :is="presetMeta(item.preset).icon" :size="16" />
              </span>
              <div class="prov-accent" />
              <div class="prov-meta">
                <span class="prov-name" :class="{ dim: !item.custom_name }">{{ item.custom_name || presetMeta(item.preset).label }}</span>
                <button
                  class="we-toggle"
                  :class="{ on: item.enabled }"
                  :disabled="presetMeta(item.preset).keyRequired && !item.api_key"
                  :title="item.enabled ? t('common.enabled') : t('common.disabled')"
                  @click.stop="toggleWebEngineExclusive(index, $event)"
                >
                  <component :is="item.enabled ? ToggleRight : ToggleLeft" :size="16" :stroke-width="1.8" />
                </button>
              </div>
            </div>
          </template>

          <template #name-input="{ item, index }">
            <div class="name-row-wrap">
              <component :is="presetMeta(item.preset).icon" :size="16" :stroke-width="1.8" class="we-name-logo" />
              <input
                v-model="item.custom_name"
                :placeholder="presetMeta(item.preset).label"
                class="fi name-fi" @click.stop
              />
              <button
                class="preset-mini-btn"
                :class="{ active: item.preset }"
                @click.stop="toggleWebPresetMenu($event, index)"
                :title="item.preset ? `${t('settings.preset')}: ${presetMeta(item.preset).label}` : t('settings.applyPreset')"
              >
                <CloudDownload :size="12" :stroke-width="1.8" />
              </button>
            </div>
          </template>

          <template #content="{ item, index }">
            <Teleport to="body">
              <Transition name="drop">
                <div v-if="showWebPresetMenu && webPresetMenuIndex === index" class="sel-menu web-preset-menu" :style="{ top: webPresetMenuPos.top + 'px', left: webPresetMenuPos.left + 'px', width: '220px' }">
                  <div class="sel-clip settings-scrollbar">
                    <button
                      v-for="p in SEARCH_PRESETS" :key="p.id"
                      class="sel-opt"
                      :class="{ hit: item.preset === p.id }"
                      @click="applyWebPreset(item, p.id)"
                    >
                      <div class="opt-left"><component :is="p.icon" :size="14" :stroke-width="1.8" />
                      <div class="opt-info">
                        <div class="opt-id-row">
                          <span class="opt-id">{{ p.label }}</span>
                        </div>
                      </div></div>
                      <Check v-if="item.preset === p.id" :size="13" :stroke-width="2.5" />
                    </button>
                  </div>
                </div>
              </Transition>
            </Teleport>
            <p v-if="presetMeta(item.preset).keyHelpKey" class="we-hint">{{ t(presetMeta(item.preset).keyHelpKey!) }}</p>
            <p v-if="presetMeta(item.preset).apiUrl" class="preset-hint" @click.stop>
              <a :href="presetMeta(item.preset).apiUrl" target="_blank" rel="noopener noreferrer" @click.prevent="openExternal(presetMeta(item.preset).apiUrl!)" style="color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px;">
                {{ t('settings.getApiKeyAt', { name: presetMeta(item.preset).label }) }}
              </a>
            </p>
            <div class="fields">
              <div class="field">
                <label>{{ t('settings.apiKey') }}</label>
                <div class="key-wrap">
                  <input
                    v-model="item.api_key"
                    :type="webEngineShowKey.has(index) ? 'text' : 'password'"
                    class="fi key-fi" @click.stop
                  />
                  <button class="icon-btn-sm" @click.stop="toggleWebEngineKeyVisible(index)" :title="t('settings.apiKey')">
                    <component :is="webEngineShowKey.has(index) ? EyeOff : Eye" :size="12" :stroke-width="1.9" />
                  </button>
                  <button
                    class="icon-btn-sm linkish"
                    @click.stop="testWebEngineConnection(item, index)"
                    :disabled="!item.api_key.trim() || getWebEngineEditState(index).testing"
                    :title="t('settings.testConnection')"
                  >
                    <Loader2 v-if="getWebEngineEditState(index).testing" :size="12" class="spin" :stroke-width="1.9" />
                    <Link2 v-else :size="12" :stroke-width="1.9" />
                  </button>
                </div>
                <Transition name="fade">
                  <span
                    v-if="getWebEngineEditState(index).status"
                    class="status-pill"
                    :class="{ ok: getWebEngineEditState(index).ok, err: !getWebEngineEditState(index).ok }"
                  >
                    <span class="status-dot" />
                    {{ getWebEngineEditState(index).status }}
                  </span>
                </Transition>
              </div>
            </div>
          </template>
          <template #disclaimer>
            <div class="api-disclaimer">
              <Info :size="11" :stroke-width="1.8" />
              <span>{{ t('settings.apiKeyDisclaimer') }}</span>
            </div>
          </template>
        </EditableCardList>

        <!-- Interface -->
        <div class="section-head mt">
          <span class="section-title"><Monitor :size="13" />{{ t('settings.interface') }}</span>
        </div>
        <div class="card-section">
          <!-- Theme -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.theme') }}</span>
            <div class="theme-toggle compact">
              <button
                v-for="opt in [{ value: 'light', icon: Sun, label: t('settings.light') }, { value: 'dark', icon: Moon, label: t('settings.dark') }, { value: 'system', icon: SunMoon, label: t('settings.system') }]"
                :key="opt.value"
                class="theme-btn"
                :class="{ on: getTheme() === opt.value }"
                @click="setTheme(opt.value as 'light' | 'dark' | 'system')"
              >
                <component :is="opt.icon" :size="13" :stroke-width="1.8" />
                {{ opt.label }}
              </button>
            </div>
          </div>
          <!-- Floating Window Opacity -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.floatingOpacity') }}</span>
            <div class="opacity-row compact">
              <Droplet :size="13" class="opacity-row-icon" />
              <input
                type="range" min="10" max="100" step="1"
                :value="appConfig.floating_opacity"
                @input="appConfig.floating_opacity = +($event.target as HTMLInputElement).value"
                class="opacity-slider"
              />
              <div class="opacity-value-wrap">
                <input
                  type="number" min="10" max="100"
                  :value="appConfig.floating_opacity"
                  @change="appConfig.floating_opacity = Math.min(100, Math.max(10, +($event.target as HTMLInputElement).value || 90))"
                  class="opacity-value-input"
                />
                <span class="opacity-pct">%</span>
                <button
                  class="opacity-reset"
                  :class="{ 'opacity-reset-off': appConfig.floating_opacity === 90 }"
                  :disabled="appConfig.floating_opacity === 90"
                  @click="appConfig.floating_opacity = 90"
                  :title="t('settings.resetToDefault')"
                >
                  <RotateCcw :size="10" :stroke-width="2" />
                </button>
              </div>
            </div>
          </div>
          <!-- Language -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.language') }}</span>
            <div class="sel-wrap compact">
              <button ref="appLangBtnRef" class="sel-btn" @click="toggleAppLangMenu()">
                <Languages :size="13" class="sel-btn-icon" />
                <span class="sel-text">{{ currentAppLangLabel }}</span>
                <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showAppLangMenu }" />
              </button>

              <Teleport to="body">
                <Transition name="drop">
                  <div v-if="showAppLangMenu" class="sel-menu" :style="{ top: appLangMenuPos.top + 'px', left: appLangMenuPos.left + 'px', width: appLangMenuPos.width + 'px' }">
                    <div class="sel-clip settings-scrollbar">
                      <button
                        v-for="opt in appLanguageOptions" :key="opt.value"
                        class="sel-opt"
                        :class="{ hit: appConfig.app_lang === opt.value }"
                        @click="selectAppLang(opt.value)"
                      >
                        <div class="opt-info">
                          <span class="opt-id">{{ opt.label }}</span>
                        </div>
                        <Check v-if="appConfig.app_lang === opt.value" :size="13" :stroke-width="2.5" />
                      </button>
                    </div>
                  </div>
                </Transition>
              </Teleport>
            </div>
          </div>
        </div>

        <!-- System -->
        <div class="section-head mt">
          <span class="section-title"><SlidersHorizontal :size="13" />{{ t('settings.systemSettings') }}</span>
        </div>
        <div class="card-section">
          <!-- Launch on Startup -->
          <div v-if="isTauri" class="card-row">
            <span class="card-label">{{ t('settings.launchOnStartup') }}</span>
            <button class="about-auto-btn" :class="{ 'toggle-on': appConfig.launch_on_startup }" @click="toggleLaunchOnStartup($event)">
              <ToggleRight v-if="appConfig.launch_on_startup" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
          </div>
          <!-- Show Shortcut Hint on Launch -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.showShortcutHintLabel') }}</span>
            <button class="about-auto-btn" :class="{ 'toggle-on': appConfig.show_startup_reminder }" @click="toggleShortcutHint($event)">
              <ToggleRight v-if="appConfig.show_startup_reminder" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
          </div>
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
                  <kbd v-for="(tok, i) in shortcutTokens" :key="i" class="kbd-badge">{{ tok }}</kbd>
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
                  <kbd v-for="(tok, i) in modeShortcutTokens" :key="i" class="kbd-badge">{{ tok }}</kbd>
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
        </div>

        <!-- History -->
        <div class="section-head mt">
          <span class="section-title"><History :size="13" />{{ t('history.historySettings') }}</span>
        </div>
        <div class="card-section" :class="{ 'remove-pending': showHistoryClearConfirm }">
          <div class="card-row">
            <span class="card-label">{{ t('history.historyLimit') }}</span>
            <div class="opacity-row compact">
              <input
                type="range" min="1" max="100" step="1"
                :value="appConfig.history_limit"
                @input="appConfig.history_limit = +($event.target as HTMLInputElement).value"
                class="opacity-slider"
              />
              <div class="opacity-value-wrap">
                <input
                  type="number" min="1" max="100"
                  :value="appConfig.history_limit"
                  @change="appConfig.history_limit = Math.min(100, Math.max(1, +($event.target as HTMLInputElement).value || 50))"
                  class="opacity-value-input"
                />
                <button
                  class="opacity-reset"
                  :class="{ 'opacity-reset-off': appConfig.history_limit === 50 }"
                  :disabled="appConfig.history_limit === 50"
                  @click="appConfig.history_limit = 50"
                  :title="t('settings.resetToDefault')"
                >
                  <RotateCcw :size="10" :stroke-width="2" />
                </button>
              </div>
            </div>
          </div>
          <div class="card-row">
            <div class="ecl-lhs">
              <template v-if="!showHistoryClearConfirm">
                <span class="card-label">{{ t('history.clearHistory') }}</span>
              </template>
              <template v-else>
                <span class="remove-warning-text">{{ t('common.cannotBeUndone') }}</span>
              </template>
            </div>
            <div class="ecl-rhs" @click.stop>
              <template v-if="!showHistoryClearConfirm">
                <button class="reset-btn" @click="showHistoryClearConfirm = true">
                  <Trash2 :size="11" :stroke-width="1.9" />{{ t('history.clearAll') }}
                </button>
              </template>
              <template v-else>
                <button class="mini-btn danger-active" :title="t('common.confirmRemove')" @click="clearAllHistory().then(() => showHistoryClearConfirm = false)">
                  <Check :size="11" :stroke-width="2.5" />
                </button>
                <button class="mini-btn" :title="t('common.cancel')" @click="showHistoryClearConfirm = false">
                  <X :size="11" :stroke-width="2.5" />
                </button>
              </template>
            </div>
          </div>
        </div>

        <!-- About -->
        <div class="section-head mt">
          <span class="section-title"><Info :size="13" />{{ t('settings.about') }}</span>
        </div>
        <div class="about-row">
          <button class="about-row-info" @click="router.push('/settings/about')">
            <img class="about-row-icon" src="/prompit_logo.svg" alt="" />
            <span class="about-row-name">Prompit</span>
            <span class="about-row-ver">v{{ appVersion }}</span>
          </button>
          <div v-if="isTauri" class="about-row-actions">
            <button
              class="pill-btn micro about-update-btn"
              :class="{
                'au-checking': updateStatus === 'checking',
                'au-ok': updateStatus === 'up-to-date',
                'au-has': updateStatus === 'has-update',
                'au-dl': updateStatus === 'preparing' || updateStatus === 'downloading',
                'au-err': updateStatus === 'error',
              }"
              :disabled="['checking','preparing','downloading','installing','restarting'].includes(updateStatus)"
              @click.stop="handleUpdateClick"
            >
              <Loader2 v-if="updateStatus === 'checking'" :size="10" class="spin" :stroke-width="2" />
              <Check v-else-if="updateStatus === 'up-to-date'" :size="10" :stroke-width="2.5" />
              <RefreshCw v-else-if="updateStatus === 'idle' || updateStatus === 'error'" :size="10" :stroke-width="2" />
              <span v-if="updateStatus === 'has-update'" class="au-ver">v{{ updateVersion }}</span>
              <span v-if="updateStatus === 'downloading' && contentLength > 0">{{ Math.round(downloaded / contentLength * 100) }}%</span>
              <span>{{ updateStatus === 'checking' ? t('about.checking')
                : updateStatus === 'up-to-date' ? t('about.upToDate')
                : updateStatus === 'has-update' ? t('about.install')
                : updateStatus === 'preparing' ? t('about.preparing')
                : updateStatus === 'downloading' ? (contentLength > 0 ? '' : t('about.downloading'))
                : updateStatus === 'installing' ? t('about.installing')
                : updateStatus === 'restarting' ? t('about.restarting')
                : updateStatus === 'error' ? (updateError || t('about.checkFailed'))
                : t('about.checkUpdate')
              }}</span>
            </button>
          </div>
        </div>
        <!-- Auto Check Update -->
        <div v-if="isTauri" class="auto-check-row">
          <span class="auto-check-label">{{ t('about.autoUpdate') }}</span>
          <button class="about-auto-btn" :class="{ 'toggle-on': autoUpdate }" @click.stop="toggleAutoUpdate($event)">
            <ToggleRight v-if="autoUpdate" :size="15" :stroke-width="1.7" />
            <ToggleLeft v-else :size="15" :stroke-width="1.7" />
          </button>
        </div>

        <!-- User Data -->
        <div class="section-head mt">
          <span class="section-title"><Database :size="13" />{{ t('settings.userData') }}</span>
        </div>
        <div class="reset-row">
          <span class="reset-desc">{{ t('settings.reset.description') }}</span>
          <button class="reset-btn" @click="router.push('/settings/reset')">
            <Trash2 :size="11" :stroke-width="1.9" />{{ t('settings.reset.button') }}
          </button>
        </div>
      </template>

      <!-- ─── Translation tab ─── -->
      <template v-if="activeTab === 'translation'">
        <!-- Model selector -->
        <div class="section-head">
          <span class="section-title"><Cpu :size="13" />{{ t('settings.translationModel') }}</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="selBtnRef"
            class="sel-btn"
            :class="{ dead: allFlat.length === 0 }"
            @click="toggleSelMenu()"
          >
            <ProviderIcon v-if="allFlat.length > 0 && translationActiveIcon" :icon="translationActiveIcon" :size="14" class="sel-icon" />
            <span class="sel-text">{{ allFlat.length === 0 ? t('settings.noModelsAvailable') : translationActiveLabel }}</span>
            <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showModelSelector }" />
          </button>

          <Teleport to="body">
            <Transition name="drop">
              <div v-if="showModelSelector && allFlat.length > 0" class="sel-menu" :style="{ top: selMenuPos.top + 'px', left: selMenuPos.left + 'px' }">
                <div class="sel-clip settings-scrollbar">
                <div class="sel-menu-inner">
                  <button
                    v-for="e in allFlat" :key="e.pIndex + '-' + e.mIndex"
                    class="sel-opt"
                    :class="{ hit: isTranslationModelActive(e.pIndex, e.mIndex) }"
                    @click="pickTranslationModel(e)"
                  >
                    <div class="opt-left"><ProviderIcon :icon="e.icon" :size="14" />
                    <div class="opt-info">
                      <span class="opt-id-row">
                        <span class="opt-id">{{ e.id }}</span>
                        <ModelCapabilityIcon :capabilities="e.input_capabilities" />
                      </span>
                      <span class="opt-src">{{ e.providerName }}</span>
                    </div></div>
                    <Check
                      v-if="isTranslationModelActive(e.pIndex, e.mIndex)"
                      :size="13" :stroke-width="2.5"
                    />
                  </button>
                </div>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- Target Language -->
        <div class="section-head mt">
          <span class="section-title"><Languages :size="13" />{{ t('settings.targetLanguage') }}</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="translationLangBtnRef"
            class="sel-btn lang-btn"
            @click="toggleTranslationLangMenu()"
          >
            <span class="sel-text">{{ getLangName(appConfig.target_lang) }}</span>
            <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: translationShowLangSelector }" />
          </button>

          <Teleport to="body">
            <Transition name="drop">
              <div v-if="translationShowLangSelector" class="sel-menu lang-menu" :style="{ top: translationLangMenuPos.top + 'px', left: translationLangMenuPos.left + 'px' }">
                <div class="sel-clip settings-scrollbar">
                <draggable
                  :list="translationLangItems"
                  item-key="id"
                  handle=".lang-drag-handle"
                  :force-fallback="true"
                  fallback-class="hidden-drag-ghost"
                  ghost-class="lang-ghost"
                  @end="onTranslationLangDragEnd"
                >
                  <template #item="{ element }">
                    <div
                      class="sel-opt lang-opt"
                      :class="{ hit: element.name === appConfig.target_lang }"
                      @click="pickTranslationLang(element.name)"
                    >
                      <span class="lang-drag-handle"><GripVertical :size="11" :stroke-width="1.8" /></span>
                      <span class="opt-label">{{ getLangName(element.name) }}</span>
                      <span class="lang-end">
                        <Check v-if="element.name === appConfig.target_lang" :size="13" :stroke-width="2.5" class="lang-item-check" />
                        <button
                          v-if="element.isCustom"
                          class="lang-item-delete"
                          @click.stop="deleteTranslationCustomLang(element.name)"
                          :title="t('settings.removeLanguage')"
                        >
                          <Trash2 :size="11" :stroke-width="1.8" />
                        </button>
                      </span>
                    </div>
                  </template>
                </draggable>

                <!-- Add language -->
                <div class="lang-sep"></div>
                <div v-if="translationShowAddLang" class="lang-add-row">
                  <input
                    v-model="translationNewLangInput"
                    class="lang-add-input"
                    :placeholder="t('settings.languageName')"
                    @keydown.enter="addTranslationCustomLang"
                    @click.stop
                    ref="translationLangAddInputRef"
                  />
                  <button class="lang-add-confirm" @click="addTranslationCustomLang" :disabled="!translationNewLangInput.trim()">
                    <Check :size="12" :stroke-width="2.5" />
                  </button>
                  <button class="lang-add-cancel" @click="translationShowAddLang = false; translationNewLangInput = ''">
                    <X :size="12" :stroke-width="2" />
                  </button>
                </div>
                <button v-else class="lang-add-btn" @click="translationShowAddLang = true">
                  <Plus :size="11" :stroke-width="2" />{{ t('settings.addLanguage') }}
                </button>

                <!-- Restore default order -->
                <button class="lang-restore-btn" @click="restoreTranslationDefaultOrder">
                  <RotateCcw :size="10" :stroke-width="1.8" />{{ t('settings.restoreTranslationDefaultOrder') }}
                </button>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- User Dictionary -->
        <div class="section-head mt">
          <span class="section-title"><BookText :size="13" />{{ t('settings.userDictionary') }}</span>
        </div>
        <div class="card-section">
          <div class="card-row">
            <span class="card-label">{{ dictStore.hasEntries ? (appConfig.user_dict_enabled ? t('common.enabled') : t('common.disabled')) : t('settings.dictEmpty') }}</span>
            <button
              class="about-auto-btn"
              :class="{ 'toggle-on': appConfig.user_dict_enabled }"
              :disabled="!dictStore.hasEntries"
              @click="toggleTranslationDict($event)"
            >
              <ToggleRight v-if="appConfig.user_dict_enabled" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
          </div>
          <div class="card-row">
            <span class="card-label">{{ t('settings.userDictionary') }}</span>
            <button
              class="mini-btn"
              :title="t('common.edit')"
              @click="router.push('/settings/dictionary?tab=translation')"
            >
              <Pencil :size="11" :stroke-width="1.9" />
            </button>
          </div>
        </div>

        <!-- Persona -->
        <EditableCardList
          id="persona-section"
          class="mt"
          :items="personaStore.personas"
          :title="t('settings.translationPersona')"
          :icon="UserCircle"
          :empty-message="t('settings.noPersonasYet')"
          :empty-sub-message="t('settings.addOneToCustomize')"
          :validate="validateTranslationPersona"
          :builtin-drag-handle="false"
          @add="Object.assign($event, { name: '', prompt: '', enabled: false })"
          @confirm="() => persistPersonas()"
          @remove="() => persistPersonas()"
        >
          <template #collapsed="{ item, index }">
            <span class="card-drag-handle prov-drag-logo" @click.stop>
              <GripVertical :size="13" :stroke-width="1.8" />
            </span>
            <button class="about-auto-btn" :class="{ 'toggle-on': item.enabled }" @click.stop="toggleTranslationPersona(index, $event)">
              <ToggleRight v-if="item.enabled" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
            <span class="persona-name">{{ item.name }}</span>
          </template>

          <template #name-input="{ item }">
            <input v-model="item.name" :placeholder="t('settings.personaName')" class="fi name-fi" @click.stop />
          </template>

          <template #content="{ item, index }">
            <div class="persona-textarea-wrap">
              <textarea
                v-model="item.prompt"
                placeholder="Enter the translation prompt for this persona…"
                class="persona-textarea"
                rows="3"
                @click.stop
                @keydown="handleTextareaKeydown($event, item, index)"
              />
              <button
                v-if="item.prompt.trim()"
                class="persona-wand-btn"
                :class="{ active: optimizingIndex === index }"
                :title="t('settings.optimizePrompt')"
                @click.stop="handleTranslationOptimizePrompt(item, index)"
              >
                <Loader2
                  v-if="optimizingIndex === index"
                  :size="12"
                  :stroke-width="1.9"
                  class="spin"
                />
                <Wand2 v-else :size="13" :stroke-width="1.6" />
              </button>
            </div>
          </template>
        </EditableCardList>
      </template>

      <!-- ─── Sparkle tab ─── -->
      <template v-if="activeTab === 'sparkle'">
        <!-- Model selector -->
        <div class="section-head">
          <span class="section-title"><Cpu :size="13" />{{ t('settings.sparkleModel') }}</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="selBtnRef"
            class="sel-btn"
            :class="{ dead: allFlat.length === 0 }"
            @click="toggleSelMenu()"
          >
            <ProviderIcon v-if="allFlat.length > 0 && sparkleActiveIcon" :icon="sparkleActiveIcon" :size="14" class="sel-icon" />
            <span class="sel-text">{{ allFlat.length === 0 ? t('settings.noModelsAvailable') : sparkleActiveLabel }}</span>
            <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showModelSelector }" />
          </button>

          <Teleport to="body">
            <Transition name="drop">
              <div v-if="showModelSelector && allFlat.length > 0" class="sel-menu" :style="{ top: selMenuPos.top + 'px', left: selMenuPos.left + 'px' }">
                <div class="sel-clip settings-scrollbar">
                <div class="sel-menu-inner">
                  <button
                    v-for="e in allFlat" :key="e.pIndex + '-' + e.mIndex"
                    class="sel-opt"
                    :class="{ hit: isSparkleModelActive(e.pIndex, e.mIndex) }"
                    @click="pickSparkleModel(e)"
                  >
                    <div class="opt-left"><ProviderIcon :icon="e.icon" :size="14" />
                    <div class="opt-info">
                      <span class="opt-id-row">
                        <span class="opt-id">{{ e.id }}</span>
                        <ModelCapabilityIcon :capabilities="e.input_capabilities" />
                      </span>
                      <span class="opt-src">{{ e.providerName }}</span>
                    </div></div>
                    <Check
                      v-if="isSparkleModelActive(e.pIndex, e.mIndex)"
                      :size="13" :stroke-width="2.5"
                    />
                  </button>
                </div>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- Sparkle card list -->
        <EditableCardList
          class="mt"
          :items="sparkleStore.sparkles"
          :title="t('settings.sparkleTitle')"
          :icon="Sparkles"
          :empty-message="t('settings.noSparklesYet')"
          :empty-sub-message="t('settings.addOneToSparkle')"
          :validate="validateSparkle"
          :allow-remove="sparkleStore.sparkles.length > 1"
          :max-collapsed="5"
          :builtin-drag-handle="false"
          @add="Object.assign($event, { name: '', prompt: '', description: '', enabled: false })"
          @confirm="() => persistSparkles()"
          @remove="() => persistSparkles()"
        >
          <template #collapsed="{ item, index }">
            <span class="card-drag-handle prov-drag-logo" @click.stop>
              <GripVertical :size="13" :stroke-width="1.8" />
            </span>
            <button class="about-auto-btn" :class="{ 'toggle-on': item.enabled }" @click.stop="toggleSparkle(index, $event)">
              <ToggleRight v-if="item.enabled" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
            <span class="sparkle-title-block">
              <span class="persona-name">{{ item.name }}</span>
              <span v-if="item.description?.trim()" class="sparkle-desc">{{ item.description }}</span>
            </span>
          </template>

          <template #name-input="{ item }">
            <input v-model="item.name" :placeholder="t('settings.sparkleName')" class="fi name-fi" @click.stop />
          </template>

          <template #content="{ item, index }">
            <div class="persona-textarea-wrap sparkle-desc-wrap">
              <input
                v-model="item.description"
                :placeholder="t('settings.sparkleDescription')"
                class="fi sparkle-desc-fi"
                @click.stop
                @keydown="handleDescKeydown($event, item, index)"
              />
              <button
                v-if="item.prompt.trim()"
                class="persona-wand-btn"
                :class="{ active: summarizingIndex === index }"
                :title="t('settings.summarizePrompt')"
                @click.stop="handleSparkleSummarize(item, index)"
              >
                <Loader2
                  v-if="summarizingIndex === index"
                  :size="12"
                  :stroke-width="1.9"
                  class="spin"
                />
                <Wand2 v-else :size="13" :stroke-width="1.6" />
              </button>
            </div>
            <div class="persona-textarea-wrap">
              <textarea
                v-model="item.prompt"
                :placeholder="t('settings.sparklePrompt')"
                class="persona-textarea"
                rows="5"
                @click.stop
                @keydown="handleTextareaKeydown($event, item, index)"
              />
              <button
                v-if="item.prompt.trim()"
                class="persona-wand-btn"
                :class="{ active: optimizingIndex === index }"
                :title="t('settings.organizePrompt')"
                @click.stop="handleSparkleOptimizePrompt(item, index)"
              >
                <Loader2
                  v-if="optimizingIndex === index"
                  :size="12"
                  :stroke-width="1.9"
                  class="spin"
                />
                <Wand2 v-else :size="13" :stroke-width="1.6" />
              </button>
            </div>
          </template>
        </EditableCardList>
      </template>
    </main>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Design tokens & base
   ══════════════════════════════════════ */
.settings-root {
  height: 100dvh; display: flex; flex-direction: column;
  background: var(--color-bg); color: var(--color-text); overflow: hidden;
  border-radius: 11px;
}
.settings-root.grow-above .settings-header { order: 2; border-bottom: none; border-top: 1px solid var(--color-surface); }
.settings-root.grow-above .tabs { order: 1; border-bottom: none; border-top: 1px solid var(--color-surface); }
.settings-root.grow-above .body { order: 0; }

/* ── Header ── */
.settings-header {
  display: flex; align-items: center; gap: 12px;
  padding: 16px 24px 12px; border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.header-title {
  flex: 1; font-size: 15px; font-weight: 700; letter-spacing: -.02em;
  color: var(--color-text); line-height: 1.2;
}
.back-btn {
  display: flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; border-radius: 9px;
  color: var(--color-text-muted); transition: .15s;
}
.back-btn:hover { color: var(--color-text); background: var(--color-surface-hover); }
.close-btn {
  display: flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; border-radius: 9px;
  color: var(--color-text-muted); transition: .15s;
}
.close-btn:hover { color: var(--color-text); background: var(--color-surface-hover); }

/* ── Tabs ── */
.tabs {
  display: flex; gap: 1px; padding: 0 24px;
  border-bottom: 1px solid var(--color-surface); flex-shrink: 0;
}
.tab {
  display: flex; align-items: center; gap: 6px;
  padding: 10px 16px; font-size: 11px; font-weight: 550;
  color: var(--color-text-muted); position: relative;
  transition: color .18s ease; cursor: default;
}
.tab::after {
  content:""; position:absolute; bottom:-1px; left:16px; right:16px;
  height: 1.5px; border-radius: 1px; background: transparent;
  transition: background .18s ease;
}
.tab:hover { color: var(--color-text-secondary); }
.tab.on { color: var(--color-accent); }
.tab.on::after { background: var(--color-accent); }

/* ── Body scroll ── */
.body {
  flex: 1; overflow-y: auto; padding: 10px 24px 16px;
}
.body::-webkit-scrollbar{width:3px}
.body::-webkit-scrollbar-thumb{background:var(--color-scrollbar);border-radius:3px}

/* ── Shared scrollbar (picker & dropdown lists) ── */
.settings-scrollbar::-webkit-scrollbar{width:3px}
.settings-scrollbar::-webkit-scrollbar-track{margin:10px 0}
.settings-scrollbar::-webkit-scrollbar-thumb{background:var(--color-scrollbar);border-radius:3px}

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
.hint {
  font-size: 9.5px; font-weight: 500; color: var(--color-text-muted);
  text-transform: uppercase; letter-spacing: .04em;
}

/* ── Pill button (Add / Fetch / Add model) ── */
.pill-btn {
  display:inline-flex; align-items:center; gap:4px;
  padding: 4px 11px; border-radius: 7px; font-size: 10.5px; font-weight: 550;
  cursor: pointer; border:none; background:none; transition:.15s;
}
.add-pill { color: var(--color-accent-text); }
.add-pill:hover { color: var(--color-accent); background: var(--color-accent-bg); }
.micro { color: var(--color-text-muted); padding: 3px 8px; }
.micro:hover:not(:disabled){ color: var(--color-text-secondary); background: var(--color-surface-hover); }
.micro:disabled{ opacity:.32; cursor:default; }
.gold-micro { color: var(--color-accent-text); }
.gold-micro:hover { color: rgba(212,160,72,.9); background: var(--color-accent-bg); }
.fetch-ok { color: var(--color-success); cursor: default; }
.fetch-ok:hover { color: var(--color-success); background: var(--color-success-bg); }

/* ── Provider collapsed content ── */
.prov-lhs { display:flex; align-items:center; gap:10px; }
/* Provider brand logo doubles as the drag handle */
.prov-drag-logo {
  display:inline-flex; align-items:center; justify-content:center;
  width: 18px; height: 26px; border-radius: 5px;
  cursor: grab; color: var(--color-text-muted);
  flex-shrink: 0;
}
.prov-drag-logo:active { cursor: grabbing; }
.prov-accent {
  width:3px; height:28px; border-radius: 2px;
  background: linear-gradient(180deg, var(--color-accent-border), rgba(212,160,72,.1));
  flex-shrink:0;
}
.prov-meta { display:flex; align-items:center; gap:8px; }
.prov-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: var(--color-text);
}
.prov-name.dim { color: var(--color-text-placeholder); font-style: italic; }
.prov-badge {
  font-size: 9.5px; font-weight: 550; color: var(--color-text-muted);
  background: var(--color-surface-hover); padding: 1px 7px; border-radius: 6px;
}
.prov-series-tag {
  flex-shrink: 0;
  font-size: 9px; font-weight: 600; letter-spacing: 0.02em;
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
  padding: 0 5px; border-radius: 4px; line-height: 16px;
  white-space: nowrap;
}
.opt-series-tag {
  flex-shrink: 0;
  font-size: 9px; font-weight: 600; letter-spacing: 0.02em;
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
  padding: 0 5px; border-radius: 4px; line-height: 16px;
  white-space: nowrap;
}


/* ── Expanded internals ── */
.name-row {
  display:flex; align-items:center; gap:7px; margin-bottom:13px;
}
.name-row-wrap {
  display:flex; align-items:center; gap:4px; flex:1; min-width:0;
}
.name-fi {
  flex:1; min-width:0;
  font-size:14px; font-weight:700; letter-spacing: -.02em;
}

.fields { display:grid; grid-template-columns:1fr; gap:10px; }
.field { display:flex; flex-direction:column; gap:4px; }

/* ── Preset mini button ── */
.preset-mini-btn {
  display: flex; align-items: center; justify-content: center;
  width: 27px; height: 27px; border-radius: 6px; flex-shrink: 0;
  color: var(--color-text-muted); cursor: pointer;
  border: none; background: none; transition: .12s;
}
.preset-mini-btn:hover { color: var(--color-accent); background: var(--color-surface-hover); }
.preset-mini-btn.active { color: var(--color-accent); }
.preset-mini-btn.active:hover { color: var(--color-accent); background: var(--color-accent-bg); }
.preset-empty {
  padding: 12px; font-size: 10.5px; color: var(--color-text-muted);
  text-align: center; font-style: italic;
}
.preset-hint {
  font-size: 10.5px; color: var(--color-text-muted);
  margin: -2px 0 8px 0; line-height: 1.4;
}

/* Variant selector (e.g. Region / Plan) for multi-variant family presets */
.variant-block {
  display: flex; flex-direction: column; gap: 6px;
  margin: -2px 0 10px 0;
}
.variant-row {
  display: flex; align-items: center; gap: 8px;
}
.variant-label {
  font-size: 9.5px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .055em; color: var(--color-text-muted);
  min-width: 52px;
}
.variant-btns { display: flex; gap: 4px; }
.variant-btn {
  font-size: 11px; padding: 3px 10px; border-radius: 6px;
  border: 1px solid var(--color-border); background: var(--color-surface);
  color: var(--color-text-muted); cursor: pointer;
  transition: border-color .15s, color .15s, background .15s;
}
.variant-btn:hover { color: var(--color-text); border-color: var(--color-accent-border); }
.variant-btn.active {
  color: var(--color-accent); border-color: var(--color-accent-border);
  background: var(--color-accent-bg);
}

label {
  font-size: 9.5px; font-weight: 600; text-transform:uppercase;
  letter-spacing: .055em; color: var(--color-text-muted);
}

.fi {
  width:100%; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius:7px;
  padding: 7px 11px; font-size: 12px; color: var(--color-text);
  outline:none; transition:border-color .15s, box-shadow .15s;
}
.fi::placeholder{ color: var(--color-text-placeholder); }
.fi:focus{ border-color: var(--color-accent-border); box-shadow: 0 0 0 2px var(--color-accent-bg); }
.fi.full { padding: 8px 12px; }

/* Key input row */
.key-wrap {
  position:relative; display:flex; align-items:center; gap:2px;
}
.key-fi { padding-right:56px !important; }
.icon-btn-sm {
  position:absolute; top:50%; transform:translateY(-50%);
  display:flex; align-items:center; justify-content:center;
  width:25px; height:25px; border-radius:5px;
  color: var(--color-text-muted); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.icon-btn-sm:nth-of-type(1){ right:27px; }
.icon-btn-sm:nth-of-type(2){ right:2px; }
.icon-btn-sm:hover:not(:disabled){ color: var(--color-text); background: var(--color-surface-hover); }
.icon-btn-sm.linkish { color: var(--color-accent-text); }
.icon-btn-sm.linkish:hover:not(:disabled){ color: var(--color-accent); background: var(--color-accent-bg); }
.icon-btn-sm:disabled{ opacity:.34; cursor:default; }

/* Status pill */
.status-pill {
  display:inline-flex; align-items:center; gap:5px;
  margin-top:5px; padding: 2px 9px 2px 7px; border-radius: 5px;
  font-size: 9.5px; font-weight: 600; letter-spacing: .01em;
}
.status-dot {
  width:5px; height:5px; border-radius:50%; flex-shrink:0;
  background: currentColor;
}
.status-pill.ok { color: var(--color-success); background: var(--color-success-bg); }
.status-pill.err { color: var(--color-danger); background: var(--color-danger-bg); }

/* Pool bar */
.pool-bar {
  display:flex; align-items:center; justify-content:space-between;
  margin-top:14px; padding-top:11px;
  border-top: 1px solid var(--color-surface);
}
.pool-label {
  font-size: 9.5px; font-weight: 600; text-transform:uppercase;
  letter-spacing: .055em; color: var(--color-text-muted);
}
.pool-actions { display:flex; align-items:center; gap:5px; }

/* ── Manual model entry ── */
.manual-model-tag {
  display:flex; align-items:center; gap:4px;
  padding: 2px 6px 2px 10px;
  border-radius: 999px; flex-shrink:0;
  background: var(--color-surface);
  border: 1px dashed var(--color-border);
}
.manual-model-input {
  width: 150px; padding: 2px 0; font-size: 10.5px; border-radius: 999px;
  background: transparent; color: var(--color-text);
  border: none; outline: none;
}
.manual-model-input::placeholder { color: var(--color-text-muted); }
.manual-model-add {
  display:flex; align-items:center; justify-content:center;
  width:20px; height:20px; border-radius:50%; flex-shrink:0;
  color: var(--color-accent); background: var(--color-accent-bg);
  border:none; cursor:pointer; transition:.12s;
}
.manual-model-add:hover:not(:disabled){ background: var(--color-accent-border); }
.manual-model-add:disabled{ opacity:.4; cursor:default; }

/* Picker (fetched models) */
.picker {
  margin-top:7px; border: 1px solid var(--color-border);
  border-radius:9px; background: var(--color-surface); overflow:hidden;
}
.picker-scroll {
  max-height:180px; overflow-y:auto; padding:3px;
}
.pick-item {
  display:flex; align-items:center; justify-content:space-between;
  width:100%; padding: 5px 9px; border-radius:5px;
  font-size: 10.5px; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  color: var(--color-text-secondary); cursor:pointer;
  border:none; background:none; text-align:left; transition:.1s;
}
.pick-item:hover:not(.dim){ background: var(--color-accent-bg); color: var(--color-text); }
.pick-item.dim{ color: var(--color-text-muted); cursor:default; }
.picker-done {
  display:block; width:100%; padding:5px; font-size:10px;
  color: var(--color-text-muted); text-align:center;
  border-top: 1px solid var(--color-surface);
  background:none; cursor:pointer; transition:color .12s;
}
.picker-done:hover{ color: var(--color-text-secondary); }

/* Tags (pool items) */
.tags {
  display:flex; flex-wrap:wrap; gap:5px; margin-top:9px;
}
.tag {
  display:inline-flex; align-items:center; gap:4px;
  padding: 3px 8px 3px 7px; border-radius:6px;
  background: var(--color-surface); border: 1px solid var(--color-surface-hover);
  font-size: 10px; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  color: var(--color-text-secondary); transition:.12s;
}
.tag:hover{ background: var(--color-surface-hover); border-color: var(--color-border); }
.tag-x {
  display:flex; align-items:center; justify-content:center;
  width:15px; height:15px; border-radius:3px;
  color: var(--color-scrollbar); cursor:pointer;
  border:none; background:none; opacity:0; transition:.1s;
}
.tag:hover .tag-x{ opacity:1; }
.tag-x:hover{ color: var(--color-danger); background: var(--color-danger-bg); }

/* Tags: pickable mode */
.tags.pickable .tag {
  cursor: pointer;
  user-select: none;
}
.tags.pickable .tag.selected {
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
  color: var(--color-accent);
}

/* Model tag drag handle */
.model-drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
  color: var(--color-text-muted);
  opacity: .55;
  transition: color .12s, opacity .12s;
}
.model-drag-handle:hover { opacity: 1; color: var(--color-text-secondary); }
.model-drag-handle:active { cursor: grabbing; }
.tag:hover .model-drag-handle { opacity: .9; }

/* Model tag drag states */
.tag.model-chosen { opacity: .35; }
.tag.model-ghost {
  opacity: .4;
  background: var(--color-accent-bg);
  border: 1px dashed var(--color-accent-border);
  color: var(--color-accent);
}

/* ── Model selector (Translation tab) ── */
.sel-wrap { position:relative; }
.sel-btn {
  display:flex; align-items:center; gap:8px; width:100%;
  padding: 9px 13px; border-radius:9px; font-size:12px;
  background: var(--color-surface); border: 1px solid var(--color-scrollbar);
  color: var(--color-text); cursor:pointer; transition:.15s; text-align:left;
}
.sel-btn:hover:not(.dead){ border-color: var(--color-border-hover); background: var(--color-surface); }
.sel-btn.dead{ color: var(--color-text-muted); cursor:default; }
.sel-text {
  flex:1; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.sel-icon { display:inline-flex; align-items:center; flex-shrink:0; }
.lang-btn .sel-text{ font-family: inherit; font-size:12px; }

.sel-arrow { color: var(--color-text-muted); transition: transform .18s; flex-shrink:0; }
.sel-arrow.rot{ transform: rotate(180deg); }

.sel-menu {
  position:fixed; min-width:230px; max-width:320px; max-height:180px;
  padding: 0; border-radius: 11px;
  background: var(--color-overlay); backdrop-filter: blur(20px) saturate(1.4);
  border: 1px solid var(--color-border);
  box-shadow: 0 16px 40px rgba(0,0,0,.55), 0 0 0 1px var(--color-surface);
  z-index:99999; overflow:hidden;
}
.sel-clip{ max-height:inherit; overflow-y:auto; overflow-x:hidden; padding:5px 7px 5px 5px; }
.sel-menu-inner{ min-height:0; }
.sel-opt {
  display:flex; align-items:center; justify-content:space-between; gap:10px;
  width:100%; padding: 8px 11px; border-radius:7px; font-size:11.5px;
  color: var(--color-text-secondary); cursor:pointer;
  border:none; background:none; text-align:left; transition:.1s;
}
.sel-opt:hover{ background: var(--color-surface-hover); color: var(--color-text); }
.sel-opt.hit{
  background: var(--color-accent-bg); color: var(--color-accent);
}
.opt-label{
  overflow:hidden; text-overflow:ellipsis; white-space:nowrap; min-width:0;
  font-size:11.5px;
}
.opt-left{ display:flex; align-items:center; gap:8px; min-width:0; flex:1; }
.opt-info{ display:flex; flex-direction:column; gap:1px; min-width:0; }
.opt-id-row{ display:flex; align-items:center; gap:5px; min-width:0; }
.opt-id{
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.opt-src{ font-size: 9px; color: var(--color-text-muted); letter-spacing: .02em; }
.lang-menu .opt-label{ font-size:12px; }
.lang-menu .sel-opt{ font-size:12px; }
.lang-menu { max-height: 340px; }
.lang-opt { gap: 4px; padding: 4px 8px; justify-content: flex-start; user-select: none; -webkit-user-select: none; }
.lang-opt .lang-drag-handle { opacity: 0; transition: opacity .12s; }
.lang-opt:hover .lang-drag-handle { opacity: 1; }
.lang-end { margin-left: auto; display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
.lang-sep { height: 1px; background: var(--color-surface-hover); margin: 4px 8px; }

/* ── Persona name (in collapsed view) ── */
.persona-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: var(--color-text);
}

/* ── Sparkle title + description (collapsed two-line hierarchy) ── */
.sparkle-title-block {
  display: flex; flex-direction: column; gap: 1px;
  min-width: 0; flex: 1;
}
.sparkle-desc {
  font-size: 11px; font-weight: 450; letter-spacing: 0;
  color: var(--color-text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.sparkle-desc-fi {
  margin-bottom: 10px;
}

/* ── Persona textarea ── */
.persona-textarea {
  width:100%; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius:7px;
  padding: 9px 11px; font-size: 12px; color: var(--color-text);
  outline:none; transition:border-color .15s, box-shadow .15s;
  resize: vertical; min-height: 60px; font-family: inherit; line-height: 1.5;
}
.persona-textarea::placeholder { color: var(--color-text-placeholder); }
.persona-textarea:focus { border-color: var(--color-accent-border); box-shadow: 0 0 0 2px var(--color-accent-bg); }
.persona-textarea { scrollbar-width: thin; scrollbar-color: var(--color-scrollbar) transparent; }
.persona-textarea::-webkit-scrollbar { width: 4px; }
.persona-textarea::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 4px; }

/* ── Persona textarea wrapper (wand button) ── */
.persona-textarea-wrap { position: relative; }
.persona-wand-btn {
  position:absolute; top:-11px; left:-11px;
  width:22px; height:22px; border-radius:50%;
  border: 1px solid var(--color-border); background:var(--color-bg);
  color:var(--color-text-muted); cursor:pointer;
  display:inline-flex; align-items:center; justify-content:center;
  opacity:0; transition:opacity .15s, color .15s, background .15s, border-color .15s; z-index:2;
  box-shadow: 0 1px 3px rgba(0,0,0,.1);
}
.persona-textarea-wrap:hover .persona-wand-btn,
.persona-wand-btn.active { opacity:1; }
.persona-wand-btn.active { color:var(--color-accent); border-color:var(--color-accent); background:color-mix(in srgb, var(--color-accent) 12%, var(--color-bg)); }
.persona-wand-btn:hover { color:var(--color-accent); border-color:var(--color-border-hover); background:color-mix(in srgb, var(--color-accent) 12%, var(--color-bg)); }
@keyframes persona-spin{ to{ transform:rotate(360deg)} }
.persona-wand-btn .spin { animation:persona-spin .75s linear infinite; }

/* ── Transitions ── */
.fade-enter-active,.fade-leave-active{ transition:opacity .18s ease; }
.fade-enter-from,.fade-leave-to{ opacity:0; }
.drop-enter-active,.drop-leave-active{ transition:opacity .14s ease,transform .14s ease; }
.drop-enter-from,.drop-leave-to{ opacity:0; transform: translateY(-5px) scale(.967); }

@keyframes spin{ to{ transform: rotate(360deg)} }
.spin{ animation: spin .75s linear infinite; }

/* ── Language dropdown management ── */
.lang-drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 4px;
  cursor: grab;
  color: var(--color-text-muted);
  flex-shrink: 0;
  transition: color 0.12s, background 0.12s;
}
.lang-drag-handle:hover {
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
}
.lang-drag-handle:active {
  cursor: grabbing;
  color: var(--color-text-secondary);
}
.lang-item-check {
  flex-shrink: 0;
  color: var(--color-accent);
}
.lang-item-delete {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 5px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.12s;
  opacity: 0;
}
.lang-opt:hover .lang-item-delete {
  opacity: 1;
}
.lang-item-delete:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.lang-ghost {
  opacity: 0.9;
  background: var(--color-accent-bg);
  border-radius: 7px;
  box-shadow: 0 4px 16px rgba(0,0,0,.3);
}
.lang-dragging {
  opacity: 0.25;
}
.hidden-drag-ghost {
  opacity: 0 !important;
  pointer-events: none !important;
}

.lang-add-row {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 5px;
}
.lang-add-input {
  flex: 1;
  padding: 5px 9px;
  border-radius: 7px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text);
  font-size: 12px;
  outline: none;
  transition: border-color 0.15s;
}
.lang-add-input:focus {
  border-color: var(--color-accent-border);
}
.lang-add-input::placeholder {
  color: var(--color-text-muted);
}
.lang-add-confirm,
.lang-add-cancel {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 7px;
  border: none;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.12s;
}
.lang-add-confirm {
  background: var(--color-accent-bg);
  color: var(--color-accent-text);
}
.lang-add-confirm:hover:not(:disabled) {
  background: var(--color-accent-border);
}
.lang-add-confirm:disabled {
  opacity: 0.25;
  cursor: default;
}
.lang-add-cancel {
  background: var(--color-surface);
  color: var(--color-text-placeholder);
}
.lang-add-cancel:hover {
  background: var(--color-scrollbar);
  color: var(--color-text-secondary);
}
.lang-add-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 6px;
  padding: 4px 9px;
  border-radius: 7px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.12s;
}
.lang-add-btn:hover {
  color: var(--color-text-secondary);
  background: var(--color-surface);
}
.lang-restore-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 6px;
  padding: 4px 9px;
  border-radius: 7px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.12s;
}
.lang-restore-btn:hover {
  color: var(--color-text-secondary);
  background: var(--color-surface);
}

/* ── Theme toggle ── */
.theme-toggle {
  display: flex;
  gap: 1px;
  background: var(--color-border);
  border-radius: 9px;
  padding: 1px;
  margin-bottom: 2px;
}
.theme-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 7px 12px;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 550;
  color: var(--color-text-muted);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
}
.theme-btn:hover {
  color: var(--color-text-secondary);
}
.theme-btn.on {
  color: var(--color-text);
  background: var(--color-surface);
  box-shadow: 0 1px 3px rgba(0,0,0,0.08);
}

/* ── Floating opacity slider ── */
.opacity-row {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 10px 14px;
}
.opacity-slider {
  flex: 1;
  -webkit-appearance: none;
  appearance: none;
  height: 4px;
  border-radius: 2px;
  background: var(--color-border);
  outline: none;
}
.opacity-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--color-accent);
  cursor: pointer;
  border: 2px solid var(--color-surface);
  box-shadow: 0 1px 4px rgba(0,0,0,0.15);
  transition: transform 0.12s ease;
}
.opacity-slider::-webkit-slider-thumb:hover {
  transform: scale(1.15);
}
.opacity-value-wrap {
  display: flex;
  align-items: center;
  gap: 2px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 3px 6px;
  flex-shrink: 0;
  width: fit-content;
}
.opacity-value-input {
  width: 28px;
  min-width: 28px;
  max-width: 28px;
  font-size: 11px;
  font-weight: 600;
  text-align: center;
  color: var(--color-text);
  background: transparent;
  border: none;
  outline: none;
  -moz-appearance: textfield;
  box-sizing: border-box;
}
.opacity-value-input::-webkit-outer-spin-button,
.opacity-value-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.opacity-pct {
  font-size: 10px;
  font-weight: 600;
  color: var(--color-text-muted);
}
.opacity-reset {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  margin-left: 2px;
  transition: all 0.12s ease;
}
.opacity-reset:not(:disabled):hover {
  background: var(--color-border);
  color: var(--color-text-secondary);
}
.opacity-reset-off {
  opacity: 0.25;
  cursor: default;
}

/* ── Card section: reusable grouped-settings container ── */
/* Usage: section-head (title) + card-section (rows inside a card) */
.card-section {
  display: flex; flex-direction: column; gap: 8px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
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
.theme-toggle.compact {
  flex: 0 1 auto;
  margin-bottom: 0;
  width: 240px;
}
.theme-toggle.compact .theme-btn {
  padding: 5px 0;
}
.opacity-row.compact {
  width: 240px;
  flex: none;
  padding: 5px 0;
  background: none;
  border: none;
}
.opacity-row-icon {
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.sel-wrap.compact {
  width: 240px;
}
.sel-wrap.compact .sel-btn {
  width: 100%;
  padding: 5px 10px;
  font-size: 11px;
}
.sel-btn-icon {
  color: var(--color-text-muted);
  flex-shrink: 0;
}

/* ── About row ── */
.about-row {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 10px 14px 10px 12px;
}

/* ── Reset row ── */
.reset-row {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 10px 14px 10px 14px;
}
.reset-desc {
  flex: 1;
  font-size: 11.5px;
  color: var(--color-text-muted);
}
.reset-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 550;
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.15s;
}
.reset-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.card-section.remove-pending { background: var(--color-danger-bg); }
.ecl-lhs { display:flex; align-items:center; gap:10px; min-width:0; flex:1; }
.ecl-rhs { display:flex; align-items:center; gap:2px; opacity:.6; transition:opacity .12s; }
.card-row:hover .ecl-rhs { opacity:1; }
.remove-warning-text {
  font-size: 10px; font-weight: 550; letter-spacing: .01em;
  color: var(--color-danger);
}
.mini-btn {
  display:flex; align-items:center; justify-content:center;
  width:27px; height:27px; border-radius:7px;
  color: var(--color-text-muted); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.mini-btn:hover { color: var(--color-text); background: var(--color-border); }
.mini-btn.danger-active {
  color: var(--color-danger); background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
@keyframes danger-pulse{ to{ background: var(--color-danger-bg)} }
.about-row-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  border: none;
  background: none;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
  transition: 0.15s;
  text-align: left;
}
.about-row-info:hover {
  background: var(--color-surface-hover);
}
.about-row-icon {
  height: 1.2em;
  width: auto;
  flex-shrink: 0;
}
.about-row-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text);
  letter-spacing: -0.01em;
}
.about-row-ver {
  font-size: 10.5px;
  color: var(--color-text-muted);
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  flex-shrink: 0;
}
.about-row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.auto-check-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding-right: 14px;
}
.auto-check-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-muted);
}
.about-update-btn {
  font-variant-numeric: tabular-nums;
  min-width: 0;
}
.about-update-btn.au-checking {
  color: var(--color-text-secondary);
  pointer-events: none;
}
.about-update-btn.au-ok {
  color: var(--color-success);
  pointer-events: none;
}
.about-update-btn.au-has {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}
.about-update-btn.au-has:hover {
  color: var(--color-accent);
}
.about-update-btn.au-dl {
  color: var(--color-accent-text);
  pointer-events: none;
}
.about-update-btn.au-err {
  color: var(--color-danger);
  pointer-events: none;
}
.au-ver {
  font-weight: 700;
}
.about-auto-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-muted);
  transition: 0.15s;
}
.about-auto-btn:hover {
  background: var(--color-surface-hover);
  color: var(--color-text);
}
.about-auto-btn.toggle-on {
  color: var(--color-accent);
  animation: toggle-pop 0.35s cubic-bezier(0.2, 0.8, 0.3, 1);
}
.about-auto-btn.toggle-on:hover {
  color: var(--color-accent);
}
.about-auto-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.about-auto-btn:disabled:hover {
  background: none;
  color: var(--color-text-muted);
}
@media (prefers-reduced-motion: reduce) {
  .about-auto-btn.toggle-on { animation: none; }
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
  opacity: 0.25;
  cursor: default;
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

/* ── Web search engine cards ──
   The collapsed row reuses the provider classes (.prov-lhs / .prov-drag-logo /
   .prov-accent / .prov-meta / .prov-name) for pixel-perfect alignment, so only
   the exclusive-toggle button needs its own rule here. */
.we-toggle {
  display:inline-flex; align-items:center; justify-content:center;
  background:none; border:none; cursor:pointer; padding:0;
  color: var(--color-text-muted); transition: color .12s;
}
.we-toggle.on { color: var(--color-accent); }
.we-toggle:disabled { opacity:.32; cursor:not-allowed; }

/* Brand logo in the name row (edit/add) — mirrors the provider name-row logo. */
.we-name-logo { color: var(--color-text-muted); flex-shrink:0; }

.we-hint {
  font-size:10.5px; line-height:1.5; color: var(--color-text-muted);
  margin:0 0 9px 0;
}

.api-disclaimer {
  display:flex; align-items:flex-start; gap:5px; flex:1; min-width:0;
  font-size:10px; line-height:1.45; color: var(--color-text-muted);
}
.api-disclaimer svg { flex-shrink:0; margin-top:1px; opacity:.65; }
</style>
