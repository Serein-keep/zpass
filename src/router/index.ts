import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/lock" },
    {
      path: "/lock",
      name: "lock",
      component: () => import("../views/LockView.vue"),
    },
    {
      path: "/main",
      name: "main",
      component: () => import("../views/MainView.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/SettingsView.vue"),
    },
  ],
});

export default router;
