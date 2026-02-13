/**
 * Worker-based Status Poller Composable
 * 
 * Bridges between the Web Worker (scheduling) and Tauri invoke() (main thread).
 * All UI state updates go through requestAnimationFrame.
 */

import { invoke } from '@tauri-apps/api/core'
import { ref, onMounted, onUnmounted, type Ref } from 'vue'

interface ProxyStatus {
  running: boolean
  socket_path?: string
}

export interface NamespaceStatus {
  namespace: string
  connected: boolean
  tool_count: number
}

interface UseWorkerPollerReturn {
  proxyStatus: Ref<ProxyStatus>
  socketAlive: Ref<boolean>
  namespaceStatuses: Ref<NamespaceStatus[]>
  /** Force an immediate poll cycle */
  refresh: () => void
}

export function useWorkerPoller(intervalMs = 15000): UseWorkerPollerReturn {
  const proxyStatus = ref<ProxyStatus>({ running: false })
  const socketAlive = ref(false)
  const namespaceStatuses = ref<NamespaceStatus[]>([])
  
  let worker: Worker | null = null
  let rafId: number | null = null
  
  function handleWorkerMessage(e: MessageEvent) {
    const msg = e.data
    
    if (msg.type === 'invoke') {
      // Bridge: Worker wants to call Tauri invoke
      invoke<unknown>(msg.command, msg.params || {})
        .then((result: unknown) => {
          worker?.postMessage({
            type: 'invoke-result',
            id: msg.id,
            result
          })
        })
        .catch((error: unknown) => {
          worker?.postMessage({
            type: 'invoke-result',
            id: msg.id,
            error: String(error)
          })
        })
    }
    else if (msg.type === 'status-update') {
      if (rafId !== null) {
        cancelAnimationFrame(rafId)
      }

      // Apply state updates in next animation frame
      rafId = requestAnimationFrame(() => {
        rafId = null
        proxyStatus.value = msg.data.proxyStatus
        socketAlive.value = msg.data.socketAlive
        namespaceStatuses.value = msg.data.namespaceStatuses
      })
    }
  }
  
  function refresh() {
    // Stop and restart to trigger immediate poll
    if (worker) {
      worker.postMessage({ type: 'stop' })
      worker.postMessage({ type: 'start', intervalMs })
    }
  }
  
  onMounted(() => {
    worker = new Worker(
      new URL('../workers/status-poller.worker.ts', import.meta.url),
      { type: 'module' }
    )
    worker.onmessage = handleWorkerMessage
    worker.onerror = (e) => {
      console.error('Status poller worker error:', e)
    }
    worker.postMessage({ type: 'start', intervalMs })
  })
  
  onUnmounted(() => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
      rafId = null
    }

    if (worker) {
      worker.postMessage({ type: 'stop' })
      worker.terminate()
      worker = null
    }
  })
  
  return {
    proxyStatus,
    socketAlive,
    namespaceStatuses,
    refresh
  }
}
