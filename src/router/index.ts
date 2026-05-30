import { createRouter, createWebHashHistory } from "vue-router";

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
  ],
});

export default router;
