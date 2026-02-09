import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

export type PermissionStatus = 'granted' | 'denied' | 'not_determined' | 'restricted' | 'unknown'

export interface Permission {
  id: string
  name: string
  description: string
  status: PermissionStatus
  critical: boolean
  last_checked?: string
}

export interface PermissionDefinition {
  id: string
  name: string
  description: string
  critical: boolean
  icon: string
  system_pref_pane: string
}

interface PermissionsCacheData {
  permissions: Permission[]
  definitions: PermissionDefinition[]
}

function mapStatus(status: string): PermissionStatus {
  const map: Record<string, PermissionStatus> = {
    granted: 'granted',
    denied: 'denied',
    not_determined: 'not_determined',
    restricted: 'restricted',
  }
  return map[status] || 'unknown'
}

export const usePermissionsStore = defineStore('permissions', () => {
  const permissions = shallowRef<Permission[]>([])
  const definitions = shallowRef<PermissionDefinition[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const hydrated = ref(false)
  const lastUpdated = ref<string | null>(null)

  const criticalPermissions = computed(() => permissions.value.filter((p) => p.critical))
  const criticalGranted = computed(() => criticalPermissions.value.filter((p) => p.status === 'granted').length)

  function persistCache() {
    const entry = saveStateCache<PermissionsCacheData>('permissions', {
      permissions: permissions.value,
      definitions: definitions.value,
    })
    lastUpdated.value = entry.lastUpdated
  }

  function hydrateFromCache() {
    const cached = loadStateCache<PermissionsCacheData>('permissions')
    if (!cached) return false
    permissions.value = cached.data.permissions ?? []
    definitions.value = cached.data.definitions ?? []
    lastUpdated.value = cached.lastUpdated
    return true
  }

  async function init() {
    if (hydrated.value) return
    hydrateFromCache()
    hydrated.value = true
    void loadDefinitions({ silent: true })
    void loadPermissions({ silent: true })
  }

  async function loadDefinitions(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true
    try {
      if (!silent && definitions.value.length === 0) loading.value = true
      const next = await invoke<PermissionDefinition[]>('get_permission_definitions')
      definitions.value = next
      persistCache()
    } catch (e) {
      console.error('Failed to load permission definitions:', e)
    } finally {
      if (!silent && definitions.value.length === 0) loading.value = false
    }
  }

  function upsertPermission(next: Permission) {
    const idx = permissions.value.findIndex((p) => p.id === next.id)
    if (idx >= 0) {
      const clone = permissions.value.slice()
      clone[idx] = { ...clone[idx], ...next }
      permissions.value = clone
    } else {
      permissions.value = [...permissions.value, next]
    }
    persistCache()
  }

  async function loadPermissions(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true
    const isInitial = permissions.value.length === 0

    if (!silent || isInitial) loading.value = true
    error.value = null

    try {
      const result = await invoke<Permission[]>('check_all_permissions')
      const mapped = result.map((p) => ({ ...p, status: mapStatus(p.status as string) }))
      permissions.value = mapped
      persistCache()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load permissions:', e)
    } finally {
      if (!silent || isInitial) loading.value = false
    }
  }

  async function checkPermission(id: string): Promise<Permission | null> {
    try {
      const result = await invoke<Permission>('check_permission', { permissionId: id })
      return { ...result, status: mapStatus(result.status as string) }
    } catch (e) {
      console.error(`Failed to check permission ${id}:`, e)
      return null
    }
  }

  async function refreshPermission(id: string): Promise<Permission | null> {
    const next = await checkPermission(id)
    if (next) upsertPermission(next)
    return next
  }

  async function requestPermission(id: string) {
    try {
      const before = permissions.value.find((p) => p.id === id)?.status
      await invoke('request_permission', { permissionId: id })

      const startedAt = Date.now()
      const maxMs = 30_000
      const intervalMs = 2_000

      while (Date.now() - startedAt < maxMs) {
        await new Promise((r) => setTimeout(r, intervalMs))
        const next = await refreshPermission(id)
        if (!next) continue
        if (before && next.status !== before) break
        if (next.status === 'granted' || next.status === 'denied') break
      }
    } catch (e) {
      console.error(`Failed to request permission ${id}:`, e)
      error.value = String(e)
    }
  }

  async function openSettings(pane: string) {
    try {
      await invoke('open_permission_settings', { pane })
    } catch (e) {
      console.error('Failed to open settings:', e)
      error.value = String(e)
    }
  }

  function getIcon(id: string): string {
    const def = definitions.value.find((d) => d.id === id)
    return def?.icon || '❓'
  }

  function getIconPath(id: string): string | null {
    const iconMap: Record<string, string> = {
      contacts: 'permission-contacts.png',
      calendar: 'permission-calendar.png',
      automation: 'permission-automation.png',
      full_disk_access: 'permission-disk.png',
      accessibility: 'permission-accessibility.png',
      screen_recording: 'permission-screen.png',
      reminders: 'permission-reminders.png',
      photos: 'permission-photos.png',
      camera: 'permission-camera.png',
      microphone: 'permission-microphone.png',
      location: 'permission-location.png',
    }

    const filename = iconMap[id]
    if (!filename) return null

    try {
      return new URL(`../assets/icons/${filename}`, import.meta.url).href
    } catch {
      return null
    }
  }

  return {
    permissions,
    definitions,
    loading,
    error,
    hydrated,
    lastUpdated,
    criticalPermissions,
    criticalGranted,
    init,
    loadDefinitions,
    loadPermissions,
    checkPermission,
    refreshPermission,
    requestPermission,
    openSettings,
    getIcon,
    getIconPath,
  }
})
