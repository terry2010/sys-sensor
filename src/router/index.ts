import { createRouter, createWebHashHistory, RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/details" },
  { path: "/details", component: () => import("../views/Details.vue") },
  { path: "/settings", component: () => import("../views/Settings.vue") },
  { path: "/about", component: () => import("../views/About.vue") },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
