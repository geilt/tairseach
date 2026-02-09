import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '../views/DashboardView.vue'
import PermissionsView from '../views/PermissionsView.vue'
import ConfigView from '../views/ConfigView.vue'
import GoogleSettingsView from '../views/GoogleSettingsView.vue'
import MonitorView from '../views/MonitorView.vue'
import ActivityView from '../views/ActivityView.vue'
import ProfilesView from '../views/ProfilesView.vue'
import AuthView from '../views/AuthView.vue'

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
      component: PermissionsView,
      meta: { title: 'Permissions' },
    },
    {
      path: '/config',
      name: 'config',
      component: ConfigView,
      meta: { title: 'Configuration' },
    },
    {
      path: '/settings/google',
      name: 'settings-google',
      component: GoogleSettingsView,
      meta: { title: 'Google OAuth Settings' },
    },
    {
      path: '/monitor',
      name: 'monitor',
      component: MonitorView,
      meta: { title: 'Monitor' },
    },
    {
      path: '/activity',
      name: 'activity',
      component: ActivityView,
      meta: { title: 'Activity Feed' },
    },
    {
      path: '/profiles',
      name: 'profiles',
      component: ProfilesView,
      meta: { title: 'Agent Profiles' },
    },
    {
      path: '/auth',
      name: 'auth',
      component: AuthView,
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
