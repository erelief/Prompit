import { watch } from "vue";
import { useColorMode } from "@vueuse/core";
import { appConfig } from "../stores/config";

// Map AppConfig.theme <-> VueUse mode
// appConfig: "light" | "dark" | "system"
// VueUse:    "light" | "dark" | "auto"
const THEME_TO_MODE: Record<string, "light" | "dark" | "auto"> = {
  light: "light",
  dark: "dark",
  system: "auto",
};
const MODE_TO_THEME: Record<string, "light" | "dark" | "system"> = {
  light: "light",
  dark: "dark",
  auto: "system",
};

const { store, system } = useColorMode({
  storageKey: null,        // we persist via Tauri config, not localStorage
  disableTransition: true, // no flash on toggle
});

/**
 * Initialize theme from appConfig.
 * Call once after loadConfig() has resolved.
 */
export function initTheme() {
  const initial = THEME_TO_MODE[appConfig.theme] || "auto";
  store.value = initial;
}

/**
 * The effective "light" | "dark" appearance,
 * after resolving "system" to the actual system preference.
 */
export function isDark() {
  // When store is "auto" (system), VueUse resolves via `system`.
  if (store.value === "auto") return system.value === "dark";
  return store.value === "dark";
}

/**
 * Set theme mode. Accepts "light" | "dark" | "system".
 */
export function setTheme(theme: "light" | "dark" | "system") {
  appConfig.theme = theme;
  store.value = THEME_TO_MODE[theme] || "auto";
}

/**
 * Current appConfig theme value ("light" | "dark" | "system").
 */
export function getTheme(): string {
  return appConfig.theme;
}

// Keep appConfig.theme in sync if VueUse mode changes externally
watch(store, (mode) => {
  const t = MODE_TO_THEME[mode] || "system";
  if (appConfig.theme !== t) {
    appConfig.theme = t;
  }
});
