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
  privacy_mode: boolean;
  translation_mode: string;
  persona: string;
}

const defaultConfig: AppConfig = {
  models: [],
  selected_model_index: 0,
  target_lang: "English",
  privacy_mode: false,
  translation_mode: "manual",
  persona: "",
};

export const appConfig = reactive<AppConfig>({ ...defaultConfig });

export async function loadConfig(): Promise<void> {
  try {
    const loaded = await invoke<AppConfig>("read_config");
    Object.assign(appConfig, loaded);
  } catch {
    // Config file doesn't exist yet — use defaults
    Object.assign(appConfig, { ...defaultConfig });
  }
}

export async function saveConfig(): Promise<void> {
  await invoke("save_config", { config: toRaw(appConfig) });
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
