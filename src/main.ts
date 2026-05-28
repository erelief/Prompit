import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./style.css";

const app = createApp(App);
app.use(router);

function applyRouteTheme(path: string) {
  const isSettings = path === "/settings";
  const bg = isSettings ? "#111827" : "transparent";
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
