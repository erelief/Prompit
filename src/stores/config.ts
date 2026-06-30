import { reactive, toRaw, watch } from "vue";
import { useTimeoutFn } from "@vueuse/core";
import { invoke } from "@tauri-apps/api/core";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { BUILTIN_LANGUAGES, LANGUAGE_GROUPS } from "../constants/languages";
import { Languages, Sparkles } from "@lucide/vue";
import i18n from "../i18n";
import type { SearchHit } from "../services/websearch/types";

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

export interface WebEngineConfig {
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
  launch_on_startup: boolean;
  show_capability_icons: boolean;
  skills_lite_active_provider_index: number;
  skills_lite_active_model_index: number;
  web_engines: WebEngineConfig[];
  web_search_active_index: number;
  web_search_enabled_in_skills_lite: boolean;
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
  app_lang: "en",
  theme: "system",
  floating_opacity: 90,
  font_size: 100,
  show_startup_reminder: true,
  history_limit: 50,
  history_enabled: true,
  shortcut: "Alt+Y",
  mode_shortcut: "Alt+M",
  launch_on_startup: false,
  show_capability_icons: false,
  skills_lite_active_provider_index: 0,
  skills_lite_active_model_index: 0,
  web_engines: [],
  web_search_active_index: -1,
  web_search_enabled_in_skills_lite: false,
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

  // Find which group contains the app language
  let appGroupKey: string | null = null;
  for (const [key, members] of Object.entries(LANGUAGE_GROUPS)) {
    if (members.includes(appLangDisplay)) {
      appGroupKey = key;
      break;
    }
  }
  if (!appGroupKey) appGroupKey = "English";

  const isEnglish = appGroupKey === "English";

  // Build ordered groups from BUILTIN_LANGUAGES, preserving original order
  const seen = new Set<string>();
  const groups: string[][] = [];
  for (const lang of BUILTIN_LANGUAGES) {
    let groupKey: string | null = null;
    for (const [key, members] of Object.entries(LANGUAGE_GROUPS)) {
      if (members.includes(lang)) { groupKey = key; break; }
    }
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

function secretKeyId(namespace: "provider" | "websearch", index: number): string {
  return `${namespace}_${index}`;
}

async function loadSecrets(): Promise<void> {
  for (let i = 0; i < appConfig.providers.length; i++) {
    try {
      const key = await invoke<string>("read_secret", {
        keyId: secretKeyId("provider", i),
      });
      if (key) {
        appConfig.providers[i].api_key = key;
      }
    } catch (err) {
      console.error(`Failed to load secret for provider ${i}:`, err);
    }
  }
  for (let i = 0; i < appConfig.web_engines.length; i++) {
    try {
      const key = await invoke<string>("read_secret", {
        keyId: secretKeyId("websearch", i),
      });
      if (key) {
        appConfig.web_engines[i].api_key = key;
      }
    } catch (err) {
      console.error(`Failed to load secret for websearch ${i}:`, err);
    }
  }
}

async function saveSecrets(): Promise<void> {
  for (let i = appConfig.providers.length; i < 50; i++) {
    try {
      await invoke("delete_secret", { keyId: secretKeyId("provider", i) });
    } catch {
      // Secret may not exist
    }
  }
  for (let i = 0; i < appConfig.providers.length; i++) {
    const apiKey = appConfig.providers[i].api_key;
    if (apiKey) {
      await invoke("save_secret", {
        keyId: secretKeyId("provider", i),
        plaintext: apiKey,
      });
    }
  }
  for (let i = appConfig.web_engines.length; i < 50; i++) {
    try {
      await invoke("delete_secret", { keyId: secretKeyId("websearch", i) });
    } catch {
      // Secret may not exist
    }
  }
  for (let i = 0; i < appConfig.web_engines.length; i++) {
    const apiKey = appConfig.web_engines[i].api_key;
    if (apiKey) {
      await invoke("save_secret", {
        keyId: secretKeyId("websearch", i),
        plaintext: apiKey,
      });
    }
  }
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
    i18n.global.locale.value = appConfig.app_lang as any;
    await loadSecrets();
    await loadPersonas();
  } catch {
    Object.assign(appConfig, { ...defaultConfig });
  }
  _configLoaded = true;
}

watch(
  () => appConfig.app_lang,
  (lang) => {
    i18n.global.locale.value = lang as any;
    rebuildLanguageOrder(lang);
  },
);

export async function saveConfig(): Promise<void> {
  await saveSecrets();

  const raw = structuredClone(toRaw(appConfig));
  for (const provider of raw.providers) {
    provider.api_key = "";
  }
  for (const engine of raw.web_engines) {
    engine.api_key = "";
  }
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
        // Strip personas from config.json by re-saving without them
        const sanitized = structuredClone(toRaw(appConfig));
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

export function getActiveModel(): {
  model: string;
  api_key: string;
  base_url: string;
  temperature: number | null;
  max_tokens: number | null;
  api_format?: ApiFormat;
} | null {
  const mode = appConfig.active_mode || "translate";
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

const DEFAULT_POLISH_SKILLS_LITE: SkillsLiteEntry = {
  name: "Polish（润色）",
  prompt:
    "Detect the language of the user's input. Adopt the role of a native speaker of that language. Rewrite the user's input as a more idiomatic, accurate, and natural expression in the same language, preserving the original meaning and intent.",
  description: "Polish the input like a native speaker of its language.",
  enabled: true,
};

export async function loadSkillsLites(): Promise<void> {
  try {
    const entries = await invoke<SkillsLiteEntry[]>("read_skills_lites");
    if (entries.length === 0) {
      skillsLiteStore.skillsLites = [DEFAULT_POLISH_SKILLS_LITE];
      await saveSkillsLites();
    } else {
      // Belt-and-suspenders with the Rust #[serde(default)]: guarantee
      // `description` is always a string even for data persisted before the field existed.
      skillsLiteStore.skillsLites = entries.map((e) => ({
        ...e,
        description: typeof e.description === "string" ? e.description : "",
      }));
    }
  } catch (err) {
    console.error("Failed to load skills lites:", err);
    skillsLiteStore.skillsLites = [DEFAULT_POLISH_SKILLS_LITE];
  }
}

export async function saveSkillsLites(): Promise<void> {
  try {
    await invoke("save_skills_lites", { skillsLites: skillsLiteStore.skillsLites });
  } catch (err) {
    console.error("Failed to save skills lites:", err);
  }
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
export interface HistoryEntry {
  input: string;
  output: string;
  timestamp: number;
  model?: string;
  mode?: string;
  persona?: string;   // active persona name (translate mode) — display only
  skills_lite?: string;   // active skills-lite name (skills_lite mode) — display only
  searched?: boolean;   // whether web search context was used (skills_lite mode)
  sources?: SearchHit[];   // web-search hits used for this entry (skills_lite mode)
  edited?: boolean;   // whether the entry was edited by the user
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

export async function saveHistoryEntry(input: string, output: string, searched: boolean = false, sources?: SearchHit[], edited: boolean = false): Promise<void> {
  if (!appConfig.history_enabled) return;
  const active = getActiveModel();
  const mode = appConfig.active_mode || "translate";
  const entry: HistoryEntry = {
    input,
    output,
    timestamp: Date.now(),
    model: active?.model || undefined,
    mode,
    searched,
    sources: sources && sources.length > 0 ? sources : undefined,
    persona: mode === "translate"
      ? (personaStore.personas.find(p => p.enabled)?.name || undefined)
      : undefined,
    skills_lite: mode === "skills_lite"
      ? (skillsLiteStore.skillsLites.find(s => s.enabled)?.name || undefined)
      : undefined,
    edited,
  };
  historyStore.entries.unshift(entry);
  const limit = appConfig.history_limit || 50;
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
  const limit = appConfig.history_limit || 50;
  try {
    await invoke("save_history", {
      entries: toRaw(historyStore.entries),
      limit,
    });
  } catch (err) {
    console.error("Failed to save history:", err);
  }
}
