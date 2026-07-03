import { createRouter, createWebHashHistory } from "vue-router";
import { appConfig, isConfigLoaded } from "../stores/config";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "floating-input",
      component: () => import("../views/FloatingInput.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/Settings.vue"),
    },
    {
      path: "/settings/dictionary",
      name: "dictionary",
      component: () => import("../views/DictionaryEditor.vue"),
    },
    {
      path: "/settings/about",
      name: "about",
      component: () => import("../views/About.vue"),
    },
    {
      path: "/settings/reset",
      name: "reset",
      component: () => import("../views/ResetSoftware.vue"),
    },
    {
      path: "/settings/data",
      name: "user-data",
      component: () => import("../views/UserData.vue"),
    },
    {
      path: "/onboarding",
      name: "onboarding",
      component: () => import("../views/Onboarding.vue"),
    },
    {
      path: "/startup-reminder",
      name: "startup-reminder",
      component: () => import("../views/StartupReminder.vue"),
    },
    {
      path: "/history",
      name: "history",
      component: () => import("../views/HistoryPanel.vue"),
    },
  ],
});

router.beforeEach((to) => {
  // During the very first navigation config isn't loaded yet, so
  // appConfig.providers is the empty default — bail out and let main.ts's
  // explicit router.replace (run after loadConfig) pick the real route.
  // Without this, every reload would be force-routed to /onboarding.
  if (!isConfigLoaded()) return;
  if (appConfig.providers.length === 0 && to.name !== "onboarding") {
    return { name: "onboarding" };
  }
  if (appConfig.providers.length > 0 && to.name === "onboarding") {
    return { name: "floating-input" };
  }
});

export default router;
