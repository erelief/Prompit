import { reactive, toRaw } from "vue";
import { invoke } from "@tauri-apps/api/core";

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
}

export interface PersonaConfig {
  name: string;
  prompt: string;
  enabled: boolean;
}

export interface AppConfig {
  providers: ProviderConfig[];
  active_provider_index: number;
  active_model_index: number;
  target_lang: string;
  personas: PersonaConfig[];
}

const defaultConfig: AppConfig = {
  providers: [],
  active_provider_index: 0,
  active_model_index: 0,
  target_lang: "English",
  personas: [],
};

export const appConfig = reactive<AppConfig>({ ...defaultConfig });

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
    await loadSecrets();
  } catch {
    Object.assign(appConfig, { ...defaultConfig });
  }
}

export async function saveConfig(): Promise<void> {
  await saveSecrets();

  const sanitized = structuredClone(toRaw(appConfig));
  for (const provider of sanitized.providers) {
    provider.api_key = "";
  }
  await invoke("save_config", { config: sanitized });
}

export function getActiveModel(): {
  model: string;
  api_key: string;
  base_url: string;
  temperature: number | null;
  max_tokens: number | null;
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
  };
}
