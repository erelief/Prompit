import { reactive, toRaw, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { BUILTIN_LANGUAGES, LANGUAGE_GROUPS } from "../constants/languages";
import i18n from "../i18n";

export interface ApiFormat {
  auth_header?: string;
  auth_prefix?: string;
  extra_headers?: Record<string, string>;
  chat_endpoint?: string;
  models_endpoint?: string;
  request?: Record<string, any>;
  response?: Record<string, string>;
}

export interface ProviderModel {
  id: string;
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

export interface ProviderPreset {
  name: string;
  provider_name: string;
  base_url: string;
  api_url: string;
  api_format: ApiFormat;
}

export interface PersonaConfig {
  name: string;
  prompt: string;
  enabled: boolean;
}

export interface DictEntry {
  source: string;
  target: string;
}

export interface AppConfig {
  providers: ProviderConfig[];
  active_provider_index: number;
  active_model_index: number;
  target_lang: string;
  user_dict_enabled: boolean;
  custom_languages: string[];
  language_order: string[];
  app_lang: string;
  theme: "light" | "dark" | "system";
  floating_opacity: number;
  show_startup_reminder: boolean;
  history_limit: number;
}

const defaultConfig: AppConfig = {
  providers: [],
  active_provider_index: 0,
  active_model_index: 0,
  target_lang: "English",
  user_dict_enabled: false,
  custom_languages: [],
  language_order: [],
  app_lang: "en",
  theme: "system",
  floating_opacity: 90,
  show_startup_reminder: true,
  history_limit: 50,
};

export const appConfig = reactive<AppConfig>({ ...defaultConfig });

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

function secretKeyId(providerIndex: number): string {
  return `provider_${providerIndex}`;
}

async function loadSecrets(): Promise<void> {
  for (let i = 0; i < appConfig.providers.length; i++) {
    try {
      const key = await invoke<string>("read_secret", {
        keyId: secretKeyId(i),
      });
      if (key) {
        appConfig.providers[i].api_key = key;
      }
    } catch (err) {
      console.error(`Failed to load secret for provider ${i}:`, err);
    }
  }
}

async function saveSecrets(): Promise<void> {
  for (let i = appConfig.providers.length; i < 50; i++) {
    try {
      await invoke("delete_secret", { keyId: secretKeyId(i) });
    } catch {
      // Secret may not exist
    }
  }
  for (let i = 0; i < appConfig.providers.length; i++) {
    const apiKey = appConfig.providers[i].api_key;
    if (apiKey) {
      await invoke("save_secret", {
        keyId: secretKeyId(i),
        plaintext: apiKey,
      });
    }
  }
}

export async function loadConfig(): Promise<void> {
  try {
    const loaded = await invoke<AppConfig>("read_config");
    Object.assign(appConfig, loaded);
    if (appConfig.target_lang === "Chinese") {
      appConfig.target_lang = "Simplified Chinese";
    }
    i18n.global.locale.value = appConfig.app_lang as any;
    await loadSecrets();
    await loadPersonas();
  } catch {
    Object.assign(appConfig, { ...defaultConfig });
  }
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

  const raw = JSON.parse(JSON.stringify(toRaw(appConfig)));
  for (const provider of raw.providers) {
    provider.api_key = "";
  }
  await invoke("save_config", { config: raw });
}

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
      }
    } catch {
      // No old config or no personas to migrate
    }
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
  const pi = appConfig.active_provider_index;
  const mi = appConfig.active_model_index;

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

// ── History ──
export interface HistoryEntry {
  input: string;
  output: string;
  timestamp: number;
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

export async function saveHistoryEntry(input: string, output: string): Promise<void> {
  const entry: HistoryEntry = {
    input,
    output,
    timestamp: Date.now(),
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
