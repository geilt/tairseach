import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

export interface Profile {
  id: string
  name: string
  type: 'agent' | 'tool' | 'mcp_server'
  config: Record<string, unknown>
  enabled: boolean
}

interface ProfilesCacheData {
  profiles: Profile[]
  activeProfile: string | null
}

export const useProfilesStore = defineStore('profiles', () => {
  const profiles = shallowRef<Profile[]>([])
  const activeProfile = ref<string | null>(null)
  const loading = ref(false)
  const hydrated = ref(false)
  const lastUpdated = ref<string | null>(null)

  function persistCache() {
    const entry = saveStateCache<ProfilesCacheData>('profiles', {
      profiles: profiles.value,
      activeProfile: activeProfile.value,
    })
    lastUpdated.value = entry.lastUpdated
  }

  function hydrateFromCache() {
    const cached = loadStateCache<ProfilesCacheData>('profiles')
    if (!cached) return
    profiles.value = cached.data.profiles ?? []
    activeProfile.value = cached.data.activeProfile ?? null
    lastUpdated.value = cached.lastUpdated
  }

  async function init() {
    if (hydrated.value) return
    hydrateFromCache()
    hydrated.value = true
    void loadProfiles({ silent: true })
  }

  async function loadProfiles(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true
    if (!silent) loading.value = true
    // TODO: Load from Tauri backend
    if (!silent) loading.value = false
    persistCache()
  }

  async function createProfile(_profile: Omit<Profile, 'id'>) {
    // TODO: Create via Tauri command
  }

  async function updateProfile(_id: string, _updates: Partial<Profile>) {
    // TODO: Update via Tauri command
  }

  async function deleteProfile(_id: string) {
    // TODO: Delete via Tauri command
  }

  async function setActiveProfile(id: string) {
    activeProfile.value = id
    persistCache()
  }

  return {
    profiles,
    activeProfile,
    loading,
    hydrated,
    lastUpdated,
    init,
    loadProfiles,
    createProfile,
    updateProfile,
    deleteProfile,
    setActiveProfile,
  }
})
