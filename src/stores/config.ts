import { reactive, toRaw } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ModelConfig {
  api_key: string;
  base_url: string;
  model: string;
  display_name: string;
  temperature: number | null;
  max_tokens: number | null;
}

export interface AppConfig {
  models: ModelConfig[];
  selected_model_index: number;
  target_lang: string;
  persona: string;
}

const defaultConfig: AppConfig = {
  models: [],
  selected_model_index: 0,
  target_lang: "English",
  persona: "",
};

export const appConfig = reactive<AppConfig>({ ...defaultConfig });

function secretKeyId(index: number): string {
  return `model_${index}`;
}

async function loadSecrets(): Promise<void> {
  for (let i = 0; i < appConfig.models.length; i++) {
    try {
      const key = await invoke<string>("read_secret", {
        keyId: secretKeyId(i),
      });
      if (key) {
        appConfig.models[i].api_key = key;
      }
    } catch (err) {
      console.error(`Failed to load secret for model ${i}:`, err);
    }
  }
}

async function saveSecrets(): Promise<void> {
  for (let i = appConfig.models.length; i < 50; i++) {
    try {
      await invoke("delete_secret", { keyId: secretKeyId(i) });
    } catch {
      // Secret may not exist
    }
  }
  for (let i = 0; i < appConfig.models.length; i++) {
    const apiKey = appConfig.models[i].api_key;
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
    await loadSecrets();
  } catch {
    Object.assign(appConfig, { ...defaultConfig });
  }
}

export async function saveConfig(): Promise<void> {
  await saveSecrets();

  const sanitized = structuredClone(toRaw(appConfig));
  for (const model of sanitized.models) {
    model.api_key = "";
  }
  await invoke("save_config", { config: sanitized });
}

export function getActiveModel(): ModelConfig | null {
  if (
    appConfig.models.length === 0 ||
    appConfig.selected_model_index >= appConfig.models.length
  ) {
    return null;
  }
  return appConfig.models[appConfig.selected_model_index];
}
