import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

export interface AuthState {
  authenticated: boolean
  method: 'none' | 'password' | 'biometric' | 'hardware_key'
  lastAuth?: Date
}

interface AuthCacheData {
  state: AuthState
}

export const useAuthStore = defineStore('auth', () => {
  const state = shallowRef<AuthState>({ authenticated: false, method: 'none' })
  const hydrated = ref(false)
  const lastUpdated = ref<string | null>(null)

  const isAuthenticated = computed(() => state.value.authenticated)

  function persistCache() {
    const entry = saveStateCache<AuthCacheData>('auth', { state: state.value })
    lastUpdated.value = entry.lastUpdated
  }

  function hydrateFromCache() {
    const cached = loadStateCache<AuthCacheData>('auth')
    if (!cached) return
    state.value = cached.data.state ?? { authenticated: false, method: 'none' }
    lastUpdated.value = cached.lastUpdated
  }

  async function init() {
    if (hydrated.value) return
    hydrateFromCache()
    hydrated.value = true
    void checkAuth()
  }

  async function authenticate(method: AuthState['method'], _credential?: string) {
    state.value = { ...state.value, authenticated: true, method, lastAuth: new Date() }
    persistCache()
  }

  async function logout() {
    state.value = { authenticated: false, method: 'none', lastAuth: undefined }
    persistCache()
  }

  async function checkAuth() {
    // TODO: Check auth state via Tauri
  }

  return {
    state,
    hydrated,
    lastUpdated,
    isAuthenticated,
    init,
    authenticate,
    logout,
    checkAuth,
  }
})
