// Shared logic for the two mode-scoped model selectors in Settings
// (translate / skills_lite). Each mode has its own pair of active-index fields
// on appConfig (`<mode>_active_provider_index` / `<mode>_active_model_index`)
// but the label/icon/pick/isActive logic is identical — only the field names
// differ. This composable reads them via dynamic keys, mirroring the pattern
// already used in FloatingInput.vue and config.ts.

import { computed, type Ref } from "vue";
import { appConfig, getProviderIcon, flushConfigSave, type ProviderPreset, type ModelInputCapabilities } from "../stores/config";

export interface FlatEntry {
  pIndex: number;
  mIndex: number;
  id: string;
  providerName: string;
  icon: string;
  input_capabilities?: ModelInputCapabilities;
}

export function useModeModelSelector(
  mode: "translate" | "skills_lite",
  showModelSelector: { value: boolean },
  providerPresets: Ref<ProviderPreset[]>,
) {
  const pKey = `${mode}_active_provider_index` as const;
  const mKey = `${mode}_active_model_index` as const;

  const label = computed(() => {
    const { providers } = appConfig;
    const pi = (appConfig as any)[pKey];
    const mi = (appConfig as any)[mKey];
    if (pi >= providers.length) return "None";
    const p = providers[pi];
    if (!p || mi >= p.models.length) return "None";
    return p.models[mi].id;
  });

  const icon = computed(() => {
    const { providers } = appConfig;
    const pi = (appConfig as any)[pKey];
    if (pi >= providers.length) return "";
    const p = providers[pi];
    return p ? getProviderIcon(p, providerPresets.value) : "";
  });

  function pick(e: FlatEntry) {
    (appConfig as any)[pKey] = e.pIndex;
    (appConfig as any)[mKey] = e.mIndex;
    showModelSelector.value = false;
    flushConfigSave();
  }

  function isActive(pIndex: number, mIndex: number): boolean {
    return pIndex === (appConfig as any)[pKey] && mIndex === (appConfig as any)[mKey];
  }

  return { label, icon, pick, isActive };
}

/** Validate a named+prompt item against siblings for duplicate names.
 *  Shared by the persona and skills-lite editors. */
export function validateNamedItem(
  arr: { name: string; prompt: string }[],
  item: { name: string; prompt: string },
  index: number,
  duplicateMsg: string,
): string | null {
  const missing: string[] = [];
  if (!item.name.trim()) missing.push("Name");
  if (!item.prompt.trim()) missing.push("Prompt");
  if (missing.length) return `Required: ${missing.join(", ")}`;
  const dup = arr.findIndex(
    (o, i) => i !== index && o.name.trim().toLowerCase() === item.name.trim().toLowerCase(),
  );
  if (dup !== -1) return duplicateMsg;
  return null;
}
