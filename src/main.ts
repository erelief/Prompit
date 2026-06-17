import { createApp } from "vue";
import { invoke } from "@tauri-apps/api/core";
import App from "./App.vue";
import router from "./router";
import i18n from "./i18n";
import { loadConfig, loadSparkles, appConfig, enableConfigAutosave } from "./stores/config";
import { initTheme } from "./composables/useTheme";
import "./style.css";

const app = createApp(App);
app.use(router);
app.use(i18n);

function applyRouteTheme(path: string) {
  const isSettings = path === "/settings" || path === "/settings/dictionary" || path === "/onboarding" || path === "/startup-reminder";
  const bg = isSettings ? "var(--color-bg)" : "transparent";
  document.documentElement.style.background = bg;
  document.body.style.background = bg;
  document.body.style.overflow = isSettings ? "auto" : "hidden";
  document.getElementById("app")!.style.background = bg;
}

router.afterEach((to) => {
  applyRouteTheme(to.path);
});

router.isReady().then(async () => {
  // Load config first so theme is known before first paint
  await loadConfig();
  // Enable debounced auto-save after initial load so the initial values
  // don't trigger an immediate write-back, and all views share one save path.
  enableConfigAutosave();
  initTheme();
  await loadSparkles();

  // Mark onboarding complete for returning users
  // (OnboardingState defaults to false on each launch; only set to true
  // inside Onboarding.vue. Without this, the global shortcut is blocked.)
  if (appConfig.providers.length > 0) {
    invoke("set_onboarding_complete");
  }

  // Decide initial route based on state
  if (appConfig.providers.length === 0) {
    // First-run: show onboarding
    router.replace("/onboarding");
  } else if (appConfig.show_startup_reminder) {
    // Non-first-run with reminder enabled
    router.replace("/startup-reminder");
  }
  // else: stays on "/" (floating-input, window hidden)

  applyRouteTheme(router.currentRoute.value.path);
  app.mount("#app");

  // Show window immediately for onboarding
  if (appConfig.providers.length === 0) {
    invoke("show_onboarding_window");
  }
});
