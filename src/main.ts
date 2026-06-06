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
  const isSettings = path === "/settings" || path === "/settings/dictionary" || path === "/onboarding";
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
  applyRouteTheme(router.currentRoute.value.path);
  app.mount("#app");

  // Show window immediately if onboarding is needed
  if (appConfig.providers.length === 0) {
    invoke("show_onboarding_window");
  }
});
