import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface ActivityEntry {
  id: string
  timestamp: string
  client: string
  namespace: string
  tool: string
  status: 'success' | 'error' | 'unknown'
  operationType: 'read' | 'write' | 'destructive' | 'error'
  raw: string
}

interface BackendEvent {
  id: string
  timestamp: string
  event_type: string
  source: string
  message: string
  metadata?: Record<string, unknown>
}

function inferNamespace(tool: string, metadata?: Record<string, unknown>): string {
  const fromMeta = typeof metadata?.namespace === 'string' ? metadata.namespace : ''
  if (fromMeta) return fromMeta

  const raw = tool.toLowerCase()
  if (raw.includes('.')) return raw.split('.')[0]
  if (raw.includes('_')) return raw.split('_')[0]
  return 'system'
}

function inferOpType(tool: string, status: ActivityEntry['status']): ActivityEntry['operationType'] {
  if (status === 'error') return 'error'

  const t = tool.toLowerCase()
  if (/(delete|remove|revoke|drop|erase|destroy|reset)/.test(t)) return 'destructive'
  if (/(create|update|write|set|grant|request|import|save|send)/.test(t)) return 'write'
  return 'read'
}

function parseBackendEvent(evt: BackendEvent): ActivityEntry {
  const metadata = evt.metadata ?? {}
  const tool =
    (typeof metadata.tool === 'string' && metadata.tool) ||
    (typeof metadata.method === 'string' && metadata.method) ||
    evt.message ||
    evt.event_type ||
    'operation'

  const statusFromMeta = typeof metadata.status === 'string' ? metadata.status.toLowerCase() : ''
  const status: ActivityEntry['status'] =
    statusFromMeta.includes('error') || statusFromMeta.includes('fail')
      ? 'error'
      : statusFromMeta.includes('success') || statusFromMeta.includes('ok')
        ? 'success'
        : evt.event_type.toLowerCase().includes('error')
          ? 'error'
          : 'unknown'

  return {
    id: evt.id,
    timestamp: evt.timestamp,
    client: evt.source || 'unknown',
    namespace: inferNamespace(tool, metadata),
    tool,
    status,
    operationType: inferOpType(tool, status),
    raw: evt.message,
  }
}

export function useActivityFeed(limit = 2000) {
  const loading = ref(false)
  const entries = shallowRef<ActivityEntry[]>([])
  const lastUpdated = ref<number>(0)

  let timer: number | null = null
  let unlisten: UnlistenFn | null = null

  const namespaces = computed(() => {
    const set = new Set(entries.value.map(e => e.namespace))
    return Array.from(set).sort()
  })

  async function refresh() {
    loading.value = true
    try {
      const res = await invoke<BackendEvent[]>('get_events', { limit })
      entries.value = res.map(parseBackendEvent)
      lastUpdated.value = Date.now()
    } finally {
      loading.value = false
    }
  }

  function startPolling(ms = 1500) {
    stopPolling()
    timer = window.setInterval(() => {
      void refresh()
    }, ms)
  }

  function stopPolling() {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  onMounted(async () => {
    await refresh()
    startPolling()

    // Optional live push path from backend event bus.
    try {
      unlisten = await listen<BackendEvent>('tairseach://activity', () => {
        void refresh()
      })
    } catch {
      unlisten = null
    }
  })

  onBeforeUnmount(() => {
    stopPolling()
    if (unlisten) unlisten()
  })

  return {
    loading,
    entries,
    namespaces,
    lastUpdated,
    refresh,
  }
}
