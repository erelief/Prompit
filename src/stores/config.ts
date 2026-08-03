import { reactive, toRaw, watch } from "vue";
import { useTimeoutFn } from "@vueuse/core";
import { invoke } from "@tauri-apps/api/core";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { BUILTIN_LANGUAGES, LANGUAGE_GROUPS } from "../constants/languages";
import { Languages, Sparkles } from "@lucide/vue";
import i18n from "../i18n";
import type { SearchHit } from "../services/websearch/types";
import { PRESET_SKILLS_LITES } from "../generated/skills-lite-presets";

export interface ApiFormat {
  auth_header?: string;
  auth_prefix?: string;
  extra_headers?: Record<string, string>;
  chat_endpoint?: string;
  models_endpoint?: string;
  response?: Record<string, string>;
  system_key?: string;
  force_fields?: string[];
}

/** Multimodal INPUT capabilities of a model. Parent dimension for all input
 *  modalities. Adding a new modality = one field here + one detection case in
 *  src/services/model-capabilities.ts. Today only `image` is implemented;
 *  `audio`/`video` are reserved as future peer optional fields. */
export interface ModelInputCapabilities {
  image?: boolean;
  audio?: boolean;
  video?: boolean;
}

export interface ProviderModel {
  id: string;
  input_capabilities?: ModelInputCapabilities;
}

export interface ProviderConfig {
  name: string;
  api_key: string;
  base_url: string;
  models: ProviderModel[];
  temperature: number | null;
  max_tokens: number | null;
  preset?: string;
  api_format?: ApiFormat;
}

export interface WebSearchProviderConfig {
  preset: string;
  api_key: string;
  enabled: boolean;
  custom_name?: string;
}

export interface PresetVariantOption {
  key: string;
  label: string;
}

/** A selectable endpoint under a region. Endpoints are scoped per-region, so
 *  their count and labels may differ across regions. */
export interface PresetVariantEndpoint {
  /** Endpoint identifier; its label is resolved via i18n key
   *  `settings.variantEndpoint_<key>`, falling back to `label` if present. */
  key: string;
  label?: string;
  /** Value written into ProviderConfig.preset AND the Name input field.
   *  Mirrors the role of the top-level `provider_name` field. */
  provider_name: string;
  base_url: string;
  api_url: string;
}

/** A region groups one or more endpoints. Endpoints vary per region. */
export interface PresetVariantRegion {
  /** Region identifier; its label is resolved via i18n key
   *  `settings.variantRegion_<key>`, falling back to `label` if present. */
  key: string;
  label?: string;
  endpoints: PresetVariantEndpoint[];
}

export interface PresetVariants {
  /** Default region key selected on first apply. */
  default_region?: string;
  /** Default endpoint key within the default region. */
  default_endpoint?: string;
  regions: PresetVariantRegion[];
}

export interface ProviderPreset {
  name: string;
  provider_name?: string;
  icon: string;
  model_series?: string[];
  base_url?: string;
  api_url?: string;
  api_format: ApiFormat;
  /** True for local-app providers (LM Studio, Ollama, …). Renders a distinct
   *  template: the hint becomes a "download" link, the API-key disclaimer is
   *  hidden, and the API-key field is optional. */
  is_local?: boolean;
  /** Optional multi-variant family: one menu entry that fans out into
   *  region → endpoint selections (e.g. CN/Global × Standard/Coding Plan).
   *  When present, provider_name/base_url/api_url are read from the selected
   *  endpoint instead, so the top-level copies may be omitted. */
  variants?: PresetVariants;
}

export interface PersonaConfig {
  name: string;
  prompt: string;
  enabled: boolean;
}

export interface SkillsLiteEntry {
  name: string;
  prompt: string;
  description: string;
  enabled: boolean;
}

export interface DictEntry {
  source: string;
  target: string;
  persona?: string;  // undefined = All (no persona constraint)
}

export interface ModeDefinition {
  id: string;
  icon: any;
  labelKey: string;
  settingTabKey: string;
}

/** History-entry limits: default value, slider bounds. */
export const HISTORY_LIMIT_DEFAULT = 100;
export const HISTORY_LIMIT_MIN = 1;
export const HISTORY_LIMIT_MAX = 500;

export interface AppConfig {
  providers: ProviderConfig[];
  active_mode: string;
  translate_active_provider_index: number;
  translate_active_model_index: number;
  target_lang: string;
  user_dict_enabled: boolean;
  custom_languages: string[];
  language_order: string[];
  app_lang: string;
  theme: "light" | "dark" | "system";
  floating_opacity: number;
  font_size: number;
  show_startup_reminder: boolean;
  history_limit: number;
  history_enabled: boolean;
  shortcut: string;
  mode_shortcut: string;
  forward_shortcut: string;
  edit_shortcut: string;
  skills_prev_shortcut: string;
  skills_next_shortcut: string;
  launch_on_startup: boolean;
  show_capability_icons: boolean;
  skills_lite_active_provider_index: number;
  skills_lite_active_model_index: number;
  web_search_providers: WebSearchProviderConfig[];
  web_search_active_index: number;
  web_search_enabled_in_skills_lite: boolean;
  webdav: WebdavSettings;
}

/**
 * WebDAV server settings (plaintext, non-sensitive). WebDAV is only a storage
 * location for the normal password-protected backup/restore flow — this holds
 * the connection fields. The account password lives in the OS credential
 * store, never here. Mirrors `WebdavSettings` in src-tauri/src/config.rs.
 */
export interface WebdavSettings {
  url: string;
  username: string;
  remote_dir: string;
  /** Upload file name (no OS save dialog exists on the WebDAV path). */
  file_name: string;
}

const defaultConfig: AppConfig = {
  providers: [],
  active_mode: "translate",
  translate_active_provider_index: 0,
  translate_active_model_index: 0,
  target_lang: "English",
  user_dict_enabled: false,
  custom_languages: [],
  language_order: [],
  app_lang: "auto",
  theme: "system",
  floating_opacity: 90,
  font_size: 100,
  show_startup_reminder: true,
  history_limit: HISTORY_LIMIT_DEFAULT,
  history_enabled: true,
  shortcut: "Alt+Y",
  mode_shortcut: "Alt+M",
  forward_shortcut: "Alt+F",
  edit_shortcut: "Alt+E",
  skills_prev_shortcut: "Alt+Up",
  skills_next_shortcut: "Alt+Down",
  launch_on_startup: false,
  show_capability_icons: false,
  skills_lite_active_provider_index: 0,
  skills_lite_active_model_index: 0,
  web_search_providers: [],
  web_search_active_index: -1,
  web_search_enabled_in_skills_lite: false,
  webdav: {
    url: "",
    username: "",
    remote_dir: "prompit",
    file_name: "prompit-backup.json",
  },
};

export const appConfig = reactive<AppConfig>({ ...defaultConfig });

// ── Centralized auto-save ──
// appConfig is a single reactive instance shared across all views, so config
// only needs to be loaded once (at startup in main.ts) and saved from one
// place. Changes are debounced (150ms) to coalesce rapid mutations (drag
// reorders, opacity slider, typing), but critical ops flush immediately.
let _saveEnabled = false;
const SAVE_DEBOUNCE_MS = 150;

// Flipped to true once loadConfig() has populated appConfig. The router guard
// uses this to avoid acting on the default (empty providers) state during the
// very first navigation, which resolves before loadConfig() runs — otherwise
// the guard would force-route every reload to /onboarding because providers
// briefly looks empty.
let _configLoaded = false;

export function isConfigLoaded(): boolean {
  return _configLoaded;
}

// Debounced save — collapses rapid bursts of config mutations into one write.
// useTimeoutFn restarts on each start() call, giving debounce semantics, and
// exposes a typed stop() for flush. immediate:false so it never fires before
// the first mutation. Created lazily so saveConfig (declared below) is in scope.
let _saveTimer: ReturnType<typeof useTimeoutFn<() => void>> | null = null;
function saveTimer() {
  if (!_saveTimer) {
    _saveTimer = useTimeoutFn(() => { void saveConfig(); }, SAVE_DEBOUNCE_MS, { immediate: false });
  }
  return _saveTimer;
}

/** Schedules a debounced save. Safe to call repeatedly; collapses bursts. */
function scheduleSave(): void {
  if (!_saveEnabled) return;
  saveTimer().start(); // restarts the timer each call → debounce
}

/** Cancels any pending debounced save and writes to disk immediately. */
export async function flushConfigSave(): Promise<void> {
  if (!_saveEnabled) return;
  if (_saveTimer) _saveTimer.stop();
  await saveConfig();
}

/** Font-size levels the user can cycle through (S / M / L / XL). */
export const FONT_SIZE_LEVELS = [85, 100, 115, 130] as const;

/** Enables debounced auto-save. Called once after initial load completes. */
export function enableConfigAutosave(): void {
  if (_saveEnabled) return;
  _saveEnabled = true;
  watch(
    () => JSON.stringify(appConfig),
    () => { scheduleSave(); },
  );
  watch(() => appConfig.font_size, (v) => {
    document.documentElement.style.setProperty('--font-scale', String((v ?? 100) / 100));
  }, { immediate: true });
}

export const dictStore = reactive({
  hasEntries: false,
});

export async function refreshDictStatus(): Promise<void> {
  const entries = await loadDictionary(appConfig.target_lang);
  dictStore.hasEntries = entries.length > 0;
  if (!dictStore.hasEntries && appConfig.user_dict_enabled) {
    appConfig.user_dict_enabled = false;
  }
}

export function getOrderedLanguages(): string[] {
  if (appConfig.language_order.length > 0) {
    return appConfig.language_order;
  }
  return [...BUILTIN_LANGUAGES, ...appConfig.custom_languages];
}

export function rebuildLanguageOrder(appLang: string): void {
  // Map BCP 47 → display name used in BUILTIN_LANGUAGES
  const BCP_TO_DISPLAY: Record<string, string> = {
    "en": "English",
    "zh-CN": "Simplified Chinese",
    "ja": "Japanese",
    "ko": "Korean",
    "fr": "French",
    "de": "German",
    "es": "Spanish",
    "ru": "Russian",
  };

  const appLangDisplay = BCP_TO_DISPLAY[appLang] || "English";

  // Find which group contains a language, or null for none.
  const groupKeyOf = (lang: string): string | null => {
    for (const [key, members] of Object.entries(LANGUAGE_GROUPS)) {
      if (members.includes(lang)) return key;
    }
    return null;
  };

  const appGroupKey = groupKeyOf(appLangDisplay) ?? "English";
  const isEnglish = appGroupKey === "English";

  // Build ordered groups from BUILTIN_LANGUAGES, preserving original order
  const seen = new Set<string>();
  const groups: string[][] = [];
  for (const lang of BUILTIN_LANGUAGES) {
    const groupKey = groupKeyOf(lang);
    if (groupKey && seen.has(groupKey)) continue;
    if (groupKey) {
      seen.add(groupKey);
      groups.push(LANGUAGE_GROUPS[groupKey]);
    } else {
      groups.push([lang]);
    }
  }

  // Find the app language's group
  const appGroupIdx = groups.findIndex(g => g.includes(appLangDisplay));
  const appGroup = appGroupIdx >= 0 ? groups.splice(appGroupIdx, 1)[0] : [];

  // Rebuild: English first if app is not English, then others, app group last
  const result: string[] = [];
  if (!isEnglish) {
    const enGroupIdx = groups.findIndex(g => LANGUAGE_GROUPS["English"]?.every(l => g.includes(l)));
    if (enGroupIdx >= 0) {
      result.push(...groups.splice(enGroupIdx, 1)[0]);
    }
  }
  for (const g of groups) result.push(...g);
  result.push(...appGroup);

  // Append custom languages (deduplicated, excluding builtins)
  const builtinSet = new Set(BUILTIN_LANGUAGES);
  for (const cl of appConfig.custom_languages) {
    if (!builtinSet.has(cl) && !result.includes(cl)) {
      result.push(cl);
    }
  }

  appConfig.language_order = result;
}

export const personaStore = reactive<{ personas: PersonaConfig[] }>({
  personas: [],
});

export const skillsLiteStore = reactive<{ skillsLites: SkillsLiteEntry[] }>({
  skillsLites: [],
});

// ── Provider & web-search engine persistence ──────────────────────────────
// Providers and web engines (with their api_key and active indices) live in
// their own encrypted files (providers.json / websearch.json), NOT in the
// plaintext config.json. The in-memory appConfig still exposes them so the rest
// of the app reads `appConfig.providers` etc. unchanged; this layer hydrates
// them after config.json loads and persists them on save.

interface ProvidersBundle {
  providers: ProviderConfig[];
  translate_active_provider_index: number;
  translate_active_model_index: number;
  skills_lite_active_provider_index: number;
  skills_lite_active_model_index: number;
}

interface WebSearchBundle {
  web_search_providers: WebSearchProviderConfig[];
  web_search_active_index: number;
}

async function loadProviders(): Promise<void> {
  try {
    const bundle = await invoke<ProvidersBundle>("read_providers_resolved");
    appConfig.providers = bundle.providers;
    appConfig.translate_active_provider_index = bundle.translate_active_provider_index;
    appConfig.translate_active_model_index = bundle.translate_active_model_index;
    appConfig.skills_lite_active_provider_index = bundle.skills_lite_active_provider_index;
    appConfig.skills_lite_active_model_index = bundle.skills_lite_active_model_index;
  } catch (err) {
    console.error("Failed to load providers bundle:", err);
  }
}

async function loadWebSearch(): Promise<void> {
  try {
    const bundle = await invoke<WebSearchBundle>("read_websearch");
    appConfig.web_search_providers = bundle.web_search_providers;
    appConfig.web_search_active_index = bundle.web_search_active_index;
  } catch (err) {
    console.error("Failed to load websearch bundle:", err);
  }
}

async function saveProviders(): Promise<void> {
  const bundle: ProvidersBundle = {
    providers: JSON.parse(JSON.stringify(toRaw(appConfig.providers))),
    translate_active_provider_index: appConfig.translate_active_provider_index,
    translate_active_model_index: appConfig.translate_active_model_index,
    skills_lite_active_provider_index: appConfig.skills_lite_active_provider_index,
    skills_lite_active_model_index: appConfig.skills_lite_active_model_index,
  };
  await invoke("save_providers", { bundle });
}

async function saveWebSearch(): Promise<void> {
  const bundle: WebSearchBundle = {
    web_search_providers: JSON.parse(JSON.stringify(toRaw(appConfig.web_search_providers))),
    web_search_active_index: appConfig.web_search_active_index,
  };
  await invoke("save_websearch", { bundle });
}

/**
 * Maps an OS locale tag (BCP 47, e.g. "zh-CN", "zh-Hans", "en-US") to one of
 * the supported `app_lang` values. Only `zh-CN` is a non-English supported
 * locale today; everything else falls back to English.
 */
function osLocaleToAppLang(osLocale: string): string {
  const tag = (osLocale || "").trim().toLowerCase();
  if (!tag) return "en";
  // Any Chinese variant (zh, zh-cn, zh-hans, zh-tw, zh-hk, …) → zh-CN.
  if (tag === "zh" || tag.startsWith("zh-") || tag.startsWith("zh_")) return "zh-CN";
  return "en";
}

/**
 * Resolves the concrete UI locale to apply. When `app_lang === "auto"` (the
 * default until the user explicitly picks a language), reads the OS UI locale
 * from `navigator.language` — which the webview derives from the system
 * preferred UI language on all platforms (WebView2/WKWebView/WebKitGTK) — and
 * maps it to a supported value. Otherwise returns the stored value as-is.
 */
function resolveAppLang(): string {
  const raw = appConfig.app_lang;
  if (raw !== "auto") return raw;
  return osLocaleToAppLang(navigator.language);
}

/**
 * Pick the release-notes block for the current UI locale.
 *
 * Release bodies are bilingual — English first, Chinese second, separated by a
 * horizontal rule (`---`), per the project's release-prompt convention. The
 * in-app "Release Notes" popup should show only the user's language. This splits
 * on `---` and returns the Chinese block when the resolved app lang is zh-CN,
 * else the first (English) block. If the body isn't split-able (older releases
 * or single-language drafts), it's returned unchanged so nothing regresses.
 *
 * The release-prompt convention appends a `**Full Changelog**: …compare/…`
 * link after the last `---`, so it lands inside the Chinese block. To keep it
 * visible in both locales, any trailing compare link is hoisted out and
 * re-appended to whichever block we return.
 */
export function pickLocalizedReleaseNotes(body: string): string {
  const text = (body ?? "").trim();
  if (!text) return "";
  // A line that is just dashes (3+) is the bilingual separator.
  let parts = text.split(/^\s*-{3,}\s*$/m).map(s => s.trim()).filter(Boolean);
  if (parts.length < 2) return text;
  // Hoist a trailing "**Full Changelog**: …" line so it shows in both locales.
  let changelog = "";
  const last = parts[parts.length - 1];
  const m = last.match(/\n?\s*(\*\*Full Changelog\*\*:\s*\S+.*)\s*$/);
  if (m) {
    changelog = m[1];
    const trimmed = last.replace(/\n?\s*\*\*Full Changelog\*\*:\s*\S+.*\s*$/, "").trim();
    parts = parts.slice(0, -1);
    if (trimmed) parts.push(trimmed);
  }
  const block = resolveAppLang() === "zh-CN" ? parts[1] : parts[0];
  return changelog ? `${block}\n\n${changelog}` : block;
}

export async function loadConfig(): Promise<void> {
  try {
    const loaded = await invoke<AppConfig>("read_config");

    // Migration: old global indices → per-mode indices
    const anyLoaded = loaded as any;
    if (anyLoaded.active_provider_index !== undefined && anyLoaded.translation_active_provider_index === undefined) {
      anyLoaded.translation_active_provider_index = anyLoaded.active_provider_index;
      anyLoaded.translation_active_model_index = anyLoaded.active_model_index;
      delete anyLoaded.active_provider_index;
      delete anyLoaded.active_model_index;
    }
    // Migration: old field name `translation_active_*` → `translate_active_*`
    // (to match the `active_mode` id "translate" used for dynamic field access).
    if (anyLoaded.translation_active_provider_index !== undefined && anyLoaded.translate_active_provider_index === undefined) {
      anyLoaded.translate_active_provider_index = anyLoaded.translation_active_provider_index;
      anyLoaded.translate_active_model_index = anyLoaded.translation_active_model_index;
      delete anyLoaded.translation_active_provider_index;
      delete anyLoaded.translation_active_model_index;
    }
    if (!anyLoaded.active_mode) {
      anyLoaded.active_mode = "translate";
    }
    // Migration: old mode id "sparkle" → "skills_lite".
    if (anyLoaded.active_mode === "sparkle") {
      anyLoaded.active_mode = "skills_lite";
    }

    Object.assign(appConfig, loaded);
    if (appConfig.target_lang === "Chinese") {
      appConfig.target_lang = "Simplified Chinese";
    }
    normalizeActiveModelIndices();
    i18n.global.locale.value = resolveAppLang() as any;
    // Providers and web engines live in their own encrypted files; hydrate them
    // AFTER Object.assign so they override any stale arrays left in config.json
    // by a partial/legacy migration.
    await loadProviders();
    await loadWebSearch();
    normalizeActiveModelIndices();
    await loadPersonas();
  } catch {
    Object.assign(appConfig, { ...defaultConfig });
  }
  _configLoaded = true;
}

watch(
  () => appConfig.app_lang,
  () => {
    // Resolve "auto" to a concrete locale (OS lookup) before applying; this
    // also fires when the user switches back to Auto in Settings/Onboarding.
    // Reads appConfig.app_lang directly (not the callback arg) so resolveAppLang
    // sees the already-updated value.
    const resolved = resolveAppLang();
    i18n.global.locale.value = resolved as any;
    rebuildLanguageOrder(resolved);
  },
);

export async function saveConfig(): Promise<void> {
  // Persist providers / web engines (with api_key) into their encrypted files.
  await saveProviders();
  await saveWebSearch();

  // Persist the slimmed config.json (theme, shortcuts, languages, etc.).
  // JSON round-trip, not structuredClone: appConfig's nested objects stay
  // reactive proxies after toRaw (which only unwraps the top level), and the
  // structured-clone algorithm throws on proxies. JSON reads through them.
  const raw = JSON.parse(JSON.stringify(toRaw(appConfig)));
  // providers[] and web_search_providers[] live in their own encrypted files now —
  // blank them in config.json so no provider data lands in plaintext.
  raw.providers = [];
  raw.web_search_providers = [];
  await invoke("save_config", { config: raw });
}

const DEFAULT_CODING_PERSONA: PersonaConfig = {
  name: "Coding（编程）",
  prompt:
    "You are a software developer with 10 years of professional experience in software engineering. You specialize in using precise, industry-standard professional software development terminology for technical communication, and your audience is cross-functional engineering teams, product managers, and technical stakeholders.",
  enabled: false,
};

export async function loadPersonas(): Promise<void> {
  try {
    const loaded = await invoke<PersonaConfig[]>("read_personas");
    if (loaded.length > 0) {
      personaStore.personas = loaded;
      return;
    }
    // Migration: read raw config.json to check for leftover personas
    try {
      const configDir = await invoke<string>("get_config_dir");
      const raw = await readTextFile(`${configDir}/config.json`);
      const parsed = JSON.parse(raw);
      if (parsed.personas && parsed.personas.length > 0) {
        personaStore.personas = parsed.personas;
        await savePersonas();
        // Strip personas from config.json by re-saving without them. Also blank
        // providers/web_search_providers — they live in encrypted files now.
        const sanitized = JSON.parse(JSON.stringify(toRaw(appConfig)));
        sanitized.personas = undefined;
        sanitized.providers = [];
        sanitized.web_search_providers = [];
        await invoke("save_config", { config: sanitized });
        return;
      }
    } catch {
      // No old config or no personas to migrate
    }
    // Nothing stored yet (fresh install): seed a reference preset the user
    // can edit or delete. Mirrors the skills-lite default-seeding behavior.
    personaStore.personas = [DEFAULT_CODING_PERSONA];
    await savePersonas();
  } catch (err) {
    console.error("Failed to load personas:", err);
  }
}

export async function savePersonas(): Promise<void> {
  try {
    await invoke("save_personas", {
      personas: toRaw(personaStore.personas),
    });
  } catch (err) {
    console.error("Failed to save personas:", err);
  }
}

export function getActiveModel(modeOverride?: string): {
  model: string;
  provider: string;
  api_key: string;
  base_url: string;
  temperature: number | null;
  max_tokens: number | null;
  api_format?: ApiFormat;
} | null {
  // Only real modes (translate / skills_lite) own per-mode model indices.
  // "summarize" and "name" are prompt-only variants used inside the
  // skills-lite card, so they resolve to the skills-lite model.
  const raw = modeOverride || appConfig.active_mode || "translate";
  const mode = raw === "summarize" || raw === "name" ? "skills_lite" : raw;
  const config = appConfig as any;
  const pi = config[`${mode}_active_provider_index`] ?? 0;
  const mi = config[`${mode}_active_model_index`] ?? 0;

  if (
    appConfig.providers.length === 0 ||
    pi >= appConfig.providers.length
  ) {
    return null;
  }

  const provider = appConfig.providers[pi];
  if (provider.models.length === 0 || mi >= provider.models.length) {
    return null;
  }

  return {
    model: provider.models[mi].id,
    provider: provider.name,
    api_key: provider.api_key,
    base_url: provider.base_url,
    temperature: provider.temperature,
    max_tokens: provider.max_tokens,
    api_format: provider.api_format,
  };
}

export async function loadDictionary(lang: string): Promise<DictEntry[]> {
  try {
    return await invoke<DictEntry[]>("read_dictionary", { targetLang: lang });
  } catch (err) {
    console.error("Failed to load dictionary:", err);
    return [];
  }
}

export async function saveDictionary(
  lang: string,
  entries: DictEntry[]
): Promise<void> {
  await invoke("save_dictionary", { targetLang: lang, entries });
}

export interface ImportResult {
  total_entries: number;
  imported: number;
  languages_affected: string[];
}

export async function importDictionaryCsv(
  filePath: string,
  mode: "add" | "overwrite"
): Promise<ImportResult> {
  return await invoke<ImportResult>("import_dictionary_csv", {
    filePath,
    mode,
  });
}

export async function exportDictionaryCsv(
  filePath: string
): Promise<void> {
  await invoke("export_dictionary_csv", {
    filePath,
  });
}

export async function clearAllDictionaries(): Promise<void> {
  await invoke("clear_all_dictionaries");
}

export async function loadProviderPresets(): Promise<ProviderPreset[]> {
  return await invoke<ProviderPreset[]>("read_provider_presets");
}

export interface ModelCapabilityItem {
  id: string;
  input_capabilities: ModelInputCapabilities;
}

export async function loadModelCapabilities(): Promise<ModelCapabilityItem[]> {
  return await invoke<ModelCapabilityItem[]>("read_model_capabilities");
}

/** Resolve a stored `preset` name back to its family preset and (if it is a
 *  variant endpoint) the specific region + endpoint. A preset name may be a
 *  top-level preset `name`, or the `provider_name` of an endpoint nested under
 *  some family's `variants.regions[].endpoints[]`. Returns `{ preset: undefined }`
 *  when nothing matches. */
export function resolvePreset(
  presetName: string | undefined,
  presets: ProviderPreset[],
): { preset?: ProviderPreset; region?: PresetVariantRegion; endpoint?: PresetVariantEndpoint } {
  if (!presetName) return {};
  // 1) endpoint match across variant families — checked FIRST so that a
  //    family whose top-level `name` collides with one of its endpoint
  //    provider_names (e.g. family "Kimi" vs endpoint "Kimi") still resolves
  //    the region/endpoint correctly instead of short-circuiting below.
  for (const p of presets) {
    for (const r of p.variants?.regions ?? []) {
      const ep = r.endpoints.find(e => e.provider_name === presetName);
      if (ep) return { preset: p, region: r, endpoint: ep };
    }
  }
  // 2) direct top-level match (plain presets without variants)
  const direct = presets.find(p => p.name === presetName);
  if (direct) return { preset: direct };
  return {};
}

export function getProviderIcon(provider: ProviderConfig, presets: ProviderPreset[]): string {
  if (!provider.preset) return ''
  return resolvePreset(provider.preset, presets).preset?.icon ?? ''
}

export function getProviderSeries(provider: ProviderConfig, presets: ProviderPreset[]): string[] {
  if (!provider.preset) return []
  return resolvePreset(provider.preset, presets).preset?.model_series ?? []
}

/** True when the provider's preset is flagged is_local (LM Studio, Ollama, …).
 *  Such providers use a distinct UI template: no API-key disclaimer, the hint
 *  is a "download" link, and the API-key field is optional. */
export function isLocalProvider(provider: ProviderConfig, presets: ProviderPreset[]): boolean {
  if (!provider.preset) return false
  return resolvePreset(provider.preset, presets).preset?.is_local ?? false
}

// ── Variant helpers (hierarchical: region → endpoints) ──

/** The i18n key for the "Region" axis label (fixed concept, not from JSON). */
export function variantRegionLabelKey(): string {
  return "settings.variantAxisRegion";
}
/** The i18n key for the "Endpoint" axis label (fixed concept, not from JSON). */
export function variantEndpointLabelKey(): string {
  return "settings.variantAxisEndpoint";
}

/** A region's display label: prefer `label`, else its i18n key, else the key. */
export function regionLabel(region: PresetVariantRegion): string {
  return region.label ?? `settings.variantRegion_${region.key}` ?? region.key;
}
/** An endpoint's display label: prefer `label`, else its i18n key, else the key. */
export function endpointLabel(endpoint: PresetVariantEndpoint): string {
  return endpoint.label ?? `settings.variantEndpoint_${endpoint.key}` ?? endpoint.key;
}

/** The default region of a variant family (variants.default_region, else first). */
export function defaultRegion(family: ProviderPreset): PresetVariantRegion | undefined {
  const v = family.variants;
  if (!v || v.regions.length === 0) return undefined;
  return v.regions.find(r => r.key === v.default_region) ?? v.regions[0];
}

/** Find a region by key within a family. */
export function findRegion(family: ProviderPreset, regionKey: string): PresetVariantRegion | undefined {
  return family.variants?.regions.find(r => r.key === regionKey);
}

/** The endpoints available under a given region (key). Empty if none. */
export function endpointsOf(family: ProviderPreset, regionKey: string): PresetVariantEndpoint[] {
  return findRegion(family, regionKey)?.endpoints ?? [];
}

/** The default endpoint of a region: default_endpoint if present & valid,
 *  else the region's first endpoint. */
export function defaultEndpoint(family: ProviderPreset, regionKey: string): PresetVariantEndpoint | undefined {
  const eps = endpointsOf(family, regionKey);
  if (eps.length === 0) return undefined;
  const dk = family.variants?.default_endpoint;
  return (dk ? eps.find(e => e.key === dk) : undefined) ?? eps[0];
}

/** The region→endpoint selection applied on first selecting the family. */
export function defaultSelection(family: ProviderPreset): { region?: PresetVariantRegion; endpoint?: PresetVariantEndpoint } {
  const region = defaultRegion(family);
  if (!region) return {};
  const endpoint = defaultEndpoint(family, region.key);
  return { region, endpoint };
}

/** True when `presetName` is the family's top-level name OR the name of any of
 *  its endpoints. Used to highlight the family entry in the preset menu. */
export function presetBelongsToFamily(
  presetName: string | undefined,
  family: ProviderPreset,
): boolean {
  if (!presetName) return false;
  if (family.name === presetName) return true;
  return family.variants?.regions.some(r => r.endpoints.some(e => e.provider_name === presetName)) ?? false;
}

// ── Skills Lite store ──

// Preset skills seeded on first install. Compiled at build time from
// skills-lite-presets/*.md + manifest.json by
// scripts/generate-skills-lite-presets.mjs. Order matters: the first entry
// (the first manifest entry) is the default-enabled one.

export async function loadSkillsLites(): Promise<void> {
  try {
    const entries = await invoke<SkillsLiteEntry[]>("read_skills_lites");
    if (entries.length === 0) {
      skillsLiteStore.skillsLites = PRESET_SKILLS_LITES;
      await saveSkillsLites();
    } else {
      // Belt-and-suspenders with the Rust #[serde(default)]: Guarantee
      // `description` is always a string even for data persisted before the field existed.
      skillsLiteStore.skillsLites = entries.map((e) => ({
        ...e,
        description: typeof e.description === "string" ? e.description : "",
      }));
    }
  } catch (err) {
    console.error("Failed to load skills lites:", err);
    skillsLiteStore.skillsLites = PRESET_SKILLS_LITES;
  }
}

export async function saveSkillsLites(): Promise<void> {
  try {
    await invoke("save_skills_lites", { skillsLites: skillsLiteStore.skillsLites });
  } catch (err) {
    console.error("Failed to save skills lites:", err);
  }
}

/** Export a single skills-lite entry to a plaintext .md file (docs/SKILL.md template). */
export async function exportSkillsLiteMarkdown(
  filePath: string,
  entry: SkillsLiteEntry,
): Promise<void> {
  await invoke("export_skills_lite_markdown", { filePath, entry });
}

export interface SkillsLiteImportResult {
  imported: number;
  skipped: number;
}

/** Import one or more .md skill files (append mode). Persists on the Rust side. */
export async function importSkillsLiteMarkdown(
  filePaths: string[],
): Promise<SkillsLiteImportResult> {
  return await invoke<SkillsLiteImportResult>("import_skills_lite_markdown", {
    filePaths,
  });
}

// ── Mode registry ──

export const MODES: ModeDefinition[] = [
  {
    id: "translate",
    icon: Languages,
    labelKey: "modes.translate",
    settingTabKey: "translation",
  },
  {
    id: "skills_lite",
    icon: Sparkles,
    labelKey: "modes.skillsLite",
    settingTabKey: "skills_lite",
  },
];

export function getCurrentMode(): ModeDefinition {
  return MODES.find(m => m.id === appConfig.active_mode) || MODES[0];
}

/**
 * Ensures each mode's stored active provider/model indices still point at a
 * real model. When they don't (e.g. the selected model or its provider was
 * deleted), falls back to the first available model across all providers —
 * matching the order of the flattened "model list" shown in the UI — instead
 * of leaving the mode pointing at nothing ("None" / a vanished button).
 * No-op when there are no providers or no models at all anywhere.
 */
export function normalizeActiveModelIndices(): void {
  const providers = appConfig.providers;
  if (providers.length === 0) return;
  // First provider (in flattened order) that exposes at least one model.
  const fallbackPi = providers.findIndex(p => p.models.length > 0);
  if (fallbackPi < 0) return;

  const config = appConfig as any;
  for (const mode of MODES) {
    const piKey = `${mode.id}_active_provider_index`;
    const miKey = `${mode.id}_active_model_index`;
    const pi = config[piKey] ?? 0;
    const mi = config[miKey] ?? 0;
    const prov = providers[pi];
    const valid = prov && prov.models.length > 0 && mi < prov.models.length;
    if (!valid) {
      config[piKey] = fallbackPi;
      config[miKey] = 0;
    }
  }
}

// Re-normalize indices when the provider/model structure shrinks at runtime
// (e.g. user deletes the active model in Settings) so no mode is left pointing
// at a non-existent model. Depends only on per-provider model counts + provider
// count, so reorders/renames (same counts) don't trigger it.
watch(
  () => appConfig.providers.length + ":" + appConfig.providers.map(p => p.models.length).join(","),
  () => { normalizeActiveModelIndices(); },
);

// ── History ──
/** Attachment metadata persisted on a history entry. Payload bytes live in
 *  files under the backend's history_attachments/ dir; `path` is relative to
 *  it, so the encrypted history.json stays small. */
export interface HistoryAttachment {
  name: string;
  mime: string;
  size: number;
  path: string;
}

/** Attachment payload handed to saveHistoryEntry; the backend decodes and
 *  writes it to disk (save_history_attachments) and returns HistoryAttachment
 *  metadata — only the metadata lands on the entry. */
export interface NewHistoryAttachment {
  name: string;
  mime: string;
  data_base64: string;
}

export interface HistoryEntry {
  input: string;
  output: string;
  timestamp: number;
  model?: string;
  mode?: string;
  usage?: TokenUsage;   // token usage of the request, when the provider reported it
  persona?: string;   // active persona name (translate mode) — display only
  skills_lite?: string;   // active skills-lite name (skills_lite mode) — display only
  searched?: boolean;   // whether web search context was used (skills_lite mode)
  sources?: SearchHit[];   // web-search hits used for this entry (skills_lite mode)
  edited?: boolean;   // whether the entry was edited by the user
  attachments?: HistoryAttachment[];   // files attached to the input (skills_lite mode)
}

export const historyStore = reactive<{ entries: HistoryEntry[] }>({
  entries: [],
});

export async function loadHistory(): Promise<void> {
  try {
    const entries = await invoke<HistoryEntry[]>("read_history");
    historyStore.entries = entries.sort((a, b) => b.timestamp - a.timestamp);
  } catch (err) {
    console.error("Failed to load history:", err);
    historyStore.entries = [];
  }
}

export async function saveHistoryEntry(input: string, output: string, searched: boolean = false, sources?: SearchHit[], edited: boolean = false, usage?: TokenUsage, attachments?: NewHistoryAttachment[]): Promise<void> {
  if (!appConfig.history_enabled) return;
  const active = getActiveModel();
  const mode = appConfig.active_mode || "translate";
  const timestamp = Date.now();
  // Persist attachment payloads first so the entry only carries metadata.
  // The entry timestamp names the on-disk dir, so it must be fixed up front.
  let attachmentMeta: HistoryAttachment[] | undefined;
  if (attachments && attachments.length > 0) {
    try {
      attachmentMeta = await invoke<HistoryAttachment[]>("save_history_attachments", { entryTs: timestamp, files: attachments });
    } catch (err) {
      console.error("Failed to persist attachments:", err);
    }
  }
  const entry: HistoryEntry = {
    input,
    output,
    timestamp,
    model: active?.model || undefined,
    mode,
    usage,
    searched,
    sources: sources && sources.length > 0 ? sources : undefined,
    persona: mode === "translate"
      ? (personaStore.personas.find(p => p.enabled)?.name || undefined)
      : undefined,
    skills_lite: mode === "skills_lite"
      ? (skillsLiteStore.skillsLites.find(s => s.enabled)?.name || undefined)
      : undefined,
    edited,
    attachments: attachmentMeta,
  };
  historyStore.entries.unshift(entry);
  const limit = appConfig.history_limit || HISTORY_LIMIT_DEFAULT;
  if (historyStore.entries.length > limit) {
    historyStore.entries = historyStore.entries.slice(0, limit);
  }
  try {
    await invoke("save_history", {
      entries: toRaw(historyStore.entries),
      limit,
    });
  } catch (err) {
    console.error("Failed to save history:", err);
  }
}

export async function clearAllHistory(): Promise<void> {
  historyStore.entries = [];
  try {
    await invoke("clear_history");
  } catch (err) {
    console.error("Failed to clear history:", err);
  }
}

export async function saveHistory(): Promise<void> {
  const limit = appConfig.history_limit || HISTORY_LIMIT_DEFAULT;
  try {
    await invoke("save_history", {
      entries: toRaw(historyStore.entries),
      limit,
    });
  } catch (err) {
    console.error("Failed to save history:", err);
  }
}

// ── Token usage stats ──
// Aggregated per-request usage, stored independently of history (history can
// be disabled, capped or cleared — stats must survive all three). Persisted
// encrypted as usage.json via the read_usage/save_usage commands, same pattern
// as the other data files.

/** Token usage reported by a provider for one request. All fields optional:
 *  some providers return no `usage` object at all (then only the request
 *  itself is counted), and Anthropic-style payloads lack a `total`. */
export interface TokenUsage {
  prompt?: number;
  completion?: number;
  total?: number;
}

export interface UsageRecord {
  ts: number;         // ms epoch
  mode: string;       // mode id at request time ("translate" | "skills_lite" | future modes)
  provider: string;   // provider display name at request time
  provider_key?: string;   // stable identity "name|base_url" — two providers sharing a display name still group separately; absent in pre-provider_key records, which fall back to `provider`
  model: string;      // model id
  prompt?: number;
  completion?: number;
  total?: number;
}

export const usageStore = reactive<{ records: UsageRecord[] }>({
  records: [],
});

// Keep a one-day buffer beyond the 30-day stats window so the "last 30 days"
// view bucketed by calendar day is always fully covered; the record cap is a
// safety valve against unbounded file growth.
const USAGE_RETENTION_MS = 31 * 24 * 60 * 60 * 1000;
const USAGE_MAX_RECORDS = 20000;

function pruneUsageRecords(records: UsageRecord[]): UsageRecord[] {
  const cutoff = Date.now() - USAGE_RETENTION_MS;
  const kept = records.filter((r) => r.ts >= cutoff);
  return kept.length > USAGE_MAX_RECORDS
    ? kept.slice(kept.length - USAGE_MAX_RECORDS)
    : kept;
}

let usageLoadPromise: Promise<void> | null = null;

/** Load usage records from disk. Memoized; pass `force` to re-read (e.g.
 *  after a reset or import replaced the file). */
export function loadUsage(force = false): Promise<void> {
  if (!usageLoadPromise || force) {
    usageLoadPromise = (async () => {
      try {
        const records = await invoke<UsageRecord[]>("read_usage");
        usageStore.records = pruneUsageRecords(records);
      } catch (err) {
        console.error("Failed to load usage stats:", err);
        usageStore.records = [];
      }
    })();
  }
  return usageLoadPromise;
}

/** Append one request to the stats and persist. Fire-and-forget safe: errors
 *  are logged, never thrown into the caller's request path. */
export async function recordUsage(record: UsageRecord): Promise<void> {
  try {
    await loadUsage(); // hydrate before appending so we never clobber the file
    usageStore.records.push(record);
    usageStore.records = pruneUsageRecords(usageStore.records);
    await invoke("save_usage", { records: toRaw(usageStore.records) });
  } catch (err) {
    console.error("Failed to save usage stats:", err);
  }
}

// ── Web search usage stats ──
// Same pattern as the token usage stats above: independent encrypted file
// (search_usage.json), 31-day rolling window. Searches are counted by request
// only — there is no token semantic, and only skills-lite mode searches, so
// records carry no mode field.

export interface SearchUsageRecord {
  ts: number;         // ms epoch
  provider: string;   // search provider display name at request time
  provider_key?: string;   // stable identity "preset|custom_name" — two providers sharing a display name still group separately
}

export const searchUsageStore = reactive<{ records: SearchUsageRecord[] }>({
  records: [],
});

function pruneSearchUsageRecords(records: SearchUsageRecord[]): SearchUsageRecord[] {
  const cutoff = Date.now() - USAGE_RETENTION_MS;
  const kept = records.filter((r) => r.ts >= cutoff);
  return kept.length > USAGE_MAX_RECORDS
    ? kept.slice(kept.length - USAGE_MAX_RECORDS)
    : kept;
}

let searchUsageLoadPromise: Promise<void> | null = null;

/** Load search usage records from disk. Memoized; pass `force` to re-read
 *  (e.g. after a reset or import replaced the file). */
export function loadSearchUsage(force = false): Promise<void> {
  if (!searchUsageLoadPromise || force) {
    searchUsageLoadPromise = (async () => {
      try {
        const records = await invoke<SearchUsageRecord[]>("read_search_usage");
        searchUsageStore.records = pruneSearchUsageRecords(records);
      } catch (err) {
        console.error("Failed to load search usage stats:", err);
        searchUsageStore.records = [];
      }
    })();
  }
  return searchUsageLoadPromise;
}

/** Append one search request to the stats and persist. Fire-and-forget safe:
 *  errors are logged, never thrown into the caller's search path. */
export async function recordSearchUsage(record: SearchUsageRecord): Promise<void> {
  try {
    await loadSearchUsage(); // hydrate before appending so we never clobber the file
    searchUsageStore.records.push(record);
    searchUsageStore.records = pruneSearchUsageRecords(searchUsageStore.records);
    await invoke("save_search_usage", { records: toRaw(searchUsageStore.records) });
  } catch (err) {
    console.error("Failed to save search usage stats:", err);
  }
}
