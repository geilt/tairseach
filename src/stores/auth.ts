import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

export interface AuthStatus {
  initialized: boolean
  master_key_available: boolean
  account_count: number
  gog_passphrase_set: boolean
}

export interface AccountInfo {
  provider: string
  account: string
  scopes: string[]
  expiry: string
  last_refreshed?: string
}

export interface TokenInfo {
  access_token: string
  token_type: string
  expiry: string
}

export interface TokenRecord {
  provider: string
  account: string
  client_id: string
  client_secret: string
  token_type: string
  access_token: string
  refresh_token: string
  expiry: string
  scopes: string[]
  issued_at?: string
  last_refreshed?: string
}

interface AuthCacheData {
  status: AuthStatus | null
  accounts: AccountInfo[]
  providers: string[]
}

export const useAuthStore = defineStore('auth', () => {
  const status = shallowRef<AuthStatus | null>(null)
  const accounts = shallowRef<AccountInfo[]>([])
  const providers = ref<string[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const hydrated = ref(false)
  const lastUpdated = ref<string | null>(null)

  const isInitialized = computed(() => status.value?.initialized ?? false)
  const accountCount = computed(() => status.value?.account_count ?? 0)

  function persistCache() {
    const entry = saveStateCache<AuthCacheData>('auth', {
      status: status.value,
      accounts: accounts.value,
      providers: providers.value,
    })
    lastUpdated.value = entry.lastUpdated
  }

  function hydrateFromCache() {
    const cached = loadStateCache<AuthCacheData>('auth')
    if (!cached) return false
    status.value = cached.data.status ?? null
    accounts.value = cached.data.accounts ?? []
    providers.value = cached.data.providers ?? []
    lastUpdated.value = cached.lastUpdated
    return true
  }

  async function init() {
    if (hydrated.value) return
    hydrateFromCache()
    hydrated.value = true
    void loadStatus({ silent: true })
    void loadProviders({ silent: true })
    void loadAccounts({ silent: true })
  }

  async function loadStatus(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true
    if (!silent) loading.value = true
    error.value = null
    try {
      status.value = await invoke<AuthStatus>('auth_status')
      persistCache()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load auth status:', e)
    } finally {
      if (!silent) loading.value = false
    }
  }

  async function loadProviders(_opts: { silent?: boolean } = {}) {
    try {
      providers.value = await invoke<string[]>('auth_providers')
      persistCache()
    } catch (e) {
      console.error('Failed to load auth providers:', e)
    }
  }

  async function loadAccounts(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true
    if (!silent) loading.value = true
    error.value = null
    try {
      accounts.value = await invoke<AccountInfo[]>('auth_accounts', { provider: null })
      persistCache()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load auth accounts:', e)
    } finally {
      if (!silent) loading.value = false
    }
  }

  async function getToken(provider: string, account: string, scopes?: string[]): Promise<TokenInfo | null> {
    try {
      return await invoke<TokenInfo>('auth_get_token', { provider, account, scopes })
    } catch (e) {
      error.value = String(e)
      console.error('Failed to get token:', e)
      return null
    }
  }

  async function refreshToken(provider: string, account: string): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      await invoke('auth_refresh_token', { provider, account })
      await loadAccounts({ silent: true })
      return true
    } catch (e) {
      error.value = String(e)
      console.error('Failed to refresh token:', e)
      return false
    } finally {
      loading.value = false
    }
  }

  async function revokeToken(provider: string, account: string): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      await invoke('auth_revoke_token', { provider, account })
      await loadAccounts({ silent: true })
      await loadStatus({ silent: true })
      return true
    } catch (e) {
      error.value = String(e)
      console.error('Failed to revoke token:', e)
      return false
    } finally {
      loading.value = false
    }
  }

  async function storeToken(record: Omit<TokenRecord, 'issued_at' | 'last_refreshed'>): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      const fullRecord = {
        ...record,
        issued_at: new Date().toISOString(),
        last_refreshed: '',
      }
      await invoke('auth_store_token', { record: fullRecord })
      await loadAccounts({ silent: true })
      await loadStatus({ silent: true })
      return true
    } catch (e) {
      error.value = String(e)
      console.error('Failed to store token:', e)
      return false
    } finally {
      loading.value = false
    }
  }

  return {
    status,
    accounts,
    providers,
    loading,
    error,
    hydrated,
    lastUpdated,
    isInitialized,
    accountCount,
    init,
    loadStatus,
    loadProviders,
    loadAccounts,
    getToken,
    refreshToken,
    revokeToken,
    storeToken,
  }
})
