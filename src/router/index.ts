import { createRouter, createWebHashHistory, RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/details" },
  { path: "/details", component: () => import("../views/Details.vue") },
  { path: "/settings", component: () => import("../views/Settings.vue") },
  { path: "/debug", component: () => import("../views/Debug.vue") },
  { path: "/about", component: () => import("../views/About.vue") },
  { path: "/floating", component: () => import("../views/Floating.vue") },
  { path: "/edge", component: () => import("../views/EdgePanel.vue") },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
