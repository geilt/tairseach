/**
 * Worker-based Status Poller Composable
 *
 * Uses unified worker registrations and batches UI writes with requestAnimationFrame.
 */

import { ref, onMounted, onScopeDispose, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ProxyStatus {
  running: boolean
  socket_path?: string
}

export interface NamespaceStatus {
  namespace: string
  connected: boolean
  tool_count: number
}

interface PollPayload {
  proxyStatus: ProxyStatus
  socketAlive: boolean
  namespaceStatuses: NamespaceStatus[]
}

interface UseWorkerPollerReturn {
  proxyStatus: Ref<ProxyStatus>
  socketAlive: Ref<boolean>
  namespaceStatuses: Ref<NamespaceStatus[]>
  /** Force an immediate poll cycle */
  refresh: () => void
}

const STATUS_POLL_CALLBACK_ID = 'status-poll'

function makeRegistrationId() {
  return `status-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

export function useWorkerPoller(intervalMs = 15000): UseWorkerPollerReturn {
  const proxyStatus = ref<ProxyStatus>({ running: false })
  const socketAlive = ref(false)
  const namespaceStatuses = ref<NamespaceStatus[]>([])

  let worker: Worker | null = null
  let registrationId: string | null = null
  let rafId: number | null = null

  function applyPayload(payload: PollPayload) {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
      rafId = null
    }

    rafId = requestAnimationFrame(() => {
      proxyStatus.value = payload.proxyStatus
      socketAlive.value = payload.socketAlive
      namespaceStatuses.value = payload.namespaceStatuses
      rafId = null
    })
  }

  function handleWorkerMessage(e: MessageEvent) {
    const msg = e.data

    if (msg.type === 'invoke') {
      invoke(msg.command, msg.params || {})
        .then((result: unknown) => {
          worker?.postMessage({
            type: 'invoke-result',
            requestId: msg.requestId,
            result,
          })
        })
        .catch((error: unknown) => {
          worker?.postMessage({
            type: 'invoke-result',
            requestId: msg.requestId,
            error: String(error),
          })
        })
      return
    }

    if (msg.type === 'poll-result' && msg.registrationId === registrationId && msg.callbackId === STATUS_POLL_CALLBACK_ID) {
      applyPayload(msg.payload as PollPayload)
      return
    }

    if (msg.type === 'poll-error' && msg.registrationId === registrationId) {
      console.warn('Unified poller worker error:', msg.error)
    }
  }

  function refresh() {
    if (!worker || !registrationId) return
    worker.postMessage({ type: 'trigger', registrationId })
  }

  onMounted(() => {
    worker = new Worker(new URL('../workers/unified-poller.worker.ts', import.meta.url), {
      type: 'module',
    })
    worker.onmessage = handleWorkerMessage
    worker.onerror = (e) => {
      console.error('Unified poller worker error:', e)
    }

    registrationId = makeRegistrationId()

    worker.postMessage({
      type: 'register',
      registrationId,
      intervalMs,
      callbackId: STATUS_POLL_CALLBACK_ID,
      immediate: true,
    })
  })

  onScopeDispose(() => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
      rafId = null
    }

    if (worker && registrationId) {
      worker.postMessage({ type: 'unregister', registrationId })
    }

    if (worker) {
      worker.postMessage({ type: 'stop-all' })
      worker.terminate()
      worker = null
    }

    registrationId = null
  })

  return {
    proxyStatus,
    socketAlive,
    namespaceStatuses,
    refresh,
  }
}
