import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

export interface ActivityEvent {
  id: string
  timestamp: Date
  type: 'tool_call' | 'permission_request' | 'error' | 'info'
  source: string
  message: string
  metadata?: Record<string, unknown>
}

interface MonitorCacheData {
  events: ActivityEvent[]
  connected: boolean
  paused: boolean
}

export const useMonitorStore = defineStore('monitor', () => {
  const events = shallowRef<ActivityEvent[]>([])
  const connected = ref(false)
  const paused = ref(false)
  const hydrated = ref(false)
  const lastUpdated = ref<string | null>(null)

  function persistCache() {
    const entry = saveStateCache<MonitorCacheData>('monitor', {
      events: events.value,
      connected: connected.value,
      paused: paused.value,
    })
    lastUpdated.value = entry.lastUpdated
  }

  function hydrateFromCache() {
    const cached = loadStateCache<MonitorCacheData>('monitor')
    if (!cached) return
    events.value = cached.data.events ?? []
    connected.value = cached.data.connected ?? false
    paused.value = cached.data.paused ?? false
    lastUpdated.value = cached.lastUpdated
  }

  async function init() {
    if (hydrated.value) return
    hydrateFromCache()
    hydrated.value = true
    void connect({ silent: true })
  }

  function addEvent(event: ActivityEvent) {
    const next = [event, ...events.value]
    events.value = next.length > 1000 ? next.slice(0, 1000) : next
    persistCache()
  }

  function clearEvents() {
    events.value = []
    persistCache()
  }

  function togglePause() {
    paused.value = !paused.value
    persistCache()
  }

  async function connect(_opts: { silent?: boolean } = {}) {
    connected.value = true
    persistCache()
  }

  async function disconnect() {
    connected.value = false
    persistCache()
  }

  return {
    events,
    connected,
    paused,
    hydrated,
    lastUpdated,
    init,
    addEvent,
    clearEvents,
    togglePause,
    connect,
    disconnect,
  }
})
