import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./style.css";

const app = createApp(App);
app.use(router);

function applyRouteTheme(path: string) {
  const isSettings = path === "/settings" || path === "/settings/dictionary";
  const bg = isSettings ? "#0b0b0f" : "transparent";
  document.documentElement.style.background = bg;
  document.body.style.background = bg;
  document.body.style.overflow = isSettings ? "auto" : "hidden";
  document.getElementById("app")!.style.background = bg;
}

router.afterEach((to) => {
  applyRouteTheme(to.path);
});

router.isReady().then(() => {
  applyRouteTheme(router.currentRoute.value.path);
  app.mount("#app");
});
