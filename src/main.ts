import { createApp } from "vue";
import { invoke } from "@tauri-apps/api/core";
import App from "./App.vue";
import router from "./router";
import i18n from "./i18n";
import { loadConfig, appConfig } from "./stores/config";
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
  initTheme();

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
