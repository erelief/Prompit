import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { loadConfig } from "./stores/config";
import { initTheme } from "./composables/useTheme";
import "./style.css";

const app = createApp(App);
app.use(router);

function applyRouteTheme(path: string) {
  const isSettings = path === "/settings" || path === "/settings/dictionary";
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
});
