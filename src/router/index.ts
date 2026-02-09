import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '../views/DashboardView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: DashboardView,
      meta: { title: 'Dashboard' },
    },
    {
      path: '/permissions',
      name: 'permissions',
      component: () => import('../views/PermissionsView.vue'),
      meta: { title: 'Permissions' },
    },
    {
      path: '/config',
      name: 'config',
      component: () => import('../views/ConfigView.vue'),
      meta: { title: 'Configuration' },
    },
    {
      path: '/monitor',
      name: 'monitor',
      component: () => import('../views/MonitorView.vue'),
      meta: { title: 'Monitor' },
    },
    {
      path: '/activity',
      name: 'activity',
      component: () => import('../views/ActivityView.vue'),
      meta: { title: 'Activity Feed' },
    },
    {
      path: '/profiles',
      name: 'profiles',
      component: () => import('../views/ProfilesView.vue'),
      meta: { title: 'Agent Profiles' },
    },
    {
      path: '/auth',
      name: 'auth',
      component: () => import('../views/AuthView.vue'),
      meta: { title: 'Auth Services' },
    },
  ],
})

// Update document title on navigation
router.afterEach((to) => {
  const title = to.meta.title as string
  document.title = title ? `${title} — Tairseach` : 'Tairseach'
})

export default router
