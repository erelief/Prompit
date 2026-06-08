import { createRouter, createWebHashHistory } from "vue-router";
import { appConfig } from "../stores/config";

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
      path: "/onboarding",
      name: "onboarding",
      component: () => import("../views/Onboarding.vue"),
    },
    {
      path: "/startup-reminder",
      name: "startup-reminder",
      component: () => import("../views/StartupReminder.vue"),
    },
  ],
});

router.beforeEach((to) => {
  if (appConfig.providers.length === 0 && to.name !== "onboarding") {
    return { name: "onboarding" };
  }
  if (appConfig.providers.length > 0 && to.name === "onboarding") {
    return { name: "floating-input" };
  }
});

export default router;
