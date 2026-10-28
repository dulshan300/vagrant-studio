import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

export const routes: RouteRecordRaw[] = [
  { path: '/', name: 'dashboard', component: () => import('@/pages/DashboardPage.vue') },
  { path: '/projects', name: 'projects', component: () => import('@/pages/ProjectsPage.vue') },
  { path: '/machines', name: 'machines', component: () => import('@/pages/MachinesPage.vue') },
  { path: '/boxes', name: 'boxes', component: () => import('@/pages/BoxesPage.vue') },
  { path: '/providers', name: 'providers', component: () => import('@/pages/ProvidersPage.vue') },
  {
    path: '/operations',
    name: 'operations',
    component: () => import('@/pages/OperationsPage.vue'),
  },
  { path: '/settings', name: 'settings', component: () => import('@/pages/SettingsPage.vue') },
  { path: '/:pathMatch(.*)*', redirect: '/' },
]

// Hash history: the desktop shell serves a single index.html, so deep links never hit the asset server.
export function createAppRouter() {
  return createRouter({ history: createWebHashHistory(), routes })
}
