import { ref, onUnmounted } from 'vue'
import { api } from '@/api/tairseach'

export interface ProxyStatus {
  running: boolean
  socket_path?: string
}

export interface NamespaceStatus {
  namespace: string
  connected: boolean
  tool_count: number
}

interface SocketStatus {
  alive: boolean
}

interface StatusPollerState {
  proxyStatus: ProxyStatus
  namespaceStatuses: NamespaceStatus[]
  socketAlive: boolean
}

interface StatusPollerOptions {
  interval?: number
  timeout?: number
  onUpdate?: (state: StatusPollerState) => void
  onError?: (error: string) => void
}

/**
 * Status polling composable with non-blocking behavior
 * 
 * Features:
 * - 15-second polling interval (not 5s)
 * - Sequential checks with 5s timeout per call
 * - Mutex prevents overlapping polls
 * - Automatic cleanup on unmount
 * - Graceful degradation on errors
 */
export function useStatusPoller(options: StatusPollerOptions = {}) {
  const {
    interval = 15000, // 15 seconds
    timeout = 5000,   // 5 seconds per invoke
    onUpdate,
    onError
  } = options

  const proxyStatus = ref<ProxyStatus>({ running: false })
  const namespaceStatuses = ref<NamespaceStatus[]>([])
  const socketAlive = ref(false)
  const isPolling = ref(false)
  const isActive = ref(false)

  let pollInterval: ReturnType<typeof setInterval> | null = null
  let pollInProgress = false // Mutex to prevent concurrent polls

  /**
   * Invoke with timeout - aborts if call takes too long
   */
  async function invokeWithTimeout<T>(
    command: string,
    args?: Record<string, unknown>
  ): Promise<T | null> {
    return new Promise((resolve) => {
      let timeoutId: ReturnType<typeof setTimeout> | null = null
      let completed = false

      const complete = (result: T | null) => {
        if (completed) return
        completed = true
        if (timeoutId) clearTimeout(timeoutId)
        resolve(result)
      }

      // Set timeout
      timeoutId = setTimeout(() => {
        complete(null)
      }, timeout)

      // Make the call
      api.system.invokeCommand<T>(command, args)
        .then((result) => complete(result))
        .catch((error) => {
          console.warn(`invoke(${command}) failed:`, error)
          complete(null)
        })
    })
  }

  /**
   * Check proxy status
   */
  async function checkProxyStatus(): Promise<ProxyStatus> {
    const result = await invokeWithTimeout<ProxyStatus>('get_proxy_status')
    return result ?? { running: false }
  }

  /**
   * Check socket liveness
   */
  async function checkSocketStatus(): Promise<boolean> {
    const result = await invokeWithTimeout<SocketStatus>('check_socket_alive')
    return result?.alive ?? false
  }

  /**
   * Load namespace statuses
   */
  async function loadNamespaceStatuses(): Promise<NamespaceStatus[]> {
    const result = await invokeWithTimeout<NamespaceStatus[]>('get_namespace_statuses')
    return result ?? []
  }

  /**
   * Execute a single poll cycle - sequential, not parallel
   */
  async function pollOnce() {
    if (pollInProgress) {
      console.log('[StatusPoller] Skipping poll - previous cycle still running')
      return
    }

    pollInProgress = true

    try {
      // Sequential execution to reduce socket contention
      const proxy = await checkProxyStatus()
      const socket = await checkSocketStatus()
      const namespaces = await loadNamespaceStatuses()

      // Update state
      proxyStatus.value = proxy
      socketAlive.value = socket
      namespaceStatuses.value = namespaces

      // Notify callback
      if (onUpdate) {
        onUpdate({
          proxyStatus: proxy,
          namespaceStatuses: namespaces,
          socketAlive: socket
        })
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      console.error('[StatusPoller] Poll cycle failed:', message)
      if (onError) {
        onError(message)
      }
    } finally {
      pollInProgress = false
    }
  }

  /**
   * Start polling
   */
  function start() {
    if (isActive.value) {
      console.warn('[StatusPoller] Already active')
      return
    }

    isActive.value = true
    isPolling.value = true

    // Initial poll
    pollOnce()

    // Set up interval
    pollInterval = setInterval(() => {
      if (isActive.value) {
        pollOnce()
      }
    }, interval)

    console.log(`[StatusPoller] Started (interval: ${interval}ms, timeout: ${timeout}ms)`)
  }

  /**
   * Stop polling
   */
  function stop() {
    isActive.value = false
    isPolling.value = false

    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }

    console.log('[StatusPoller] Stopped')
  }

  /**
   * Force an immediate poll (resets the interval)
   */
  function refresh() {
    if (!isActive.value) {
      console.warn('[StatusPoller] Cannot refresh - not active')
      return
    }

    // Reset interval
    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = setInterval(() => {
        if (isActive.value) {
          pollOnce()
        }
      }, interval)
    }

    // Poll immediately
    pollOnce()
  }

  // Cleanup on unmount
  onUnmounted(() => {
    stop()
  })

  return {
    // State
    proxyStatus,
    namespaceStatuses,
    socketAlive,
    isPolling,
    
    // Controls
    start,
    stop,
    refresh,
    
    // Manual poll (useful for initial load)
    pollOnce
  }
}
