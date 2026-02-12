/**
 * Status Poller Web Worker
 * 
 * Owns all scheduling/timing logic for MCP status polling.
 * Communicates with main thread via message passing for Tauri invoke() calls.
 */

interface InvokeRequest {
  type: 'invoke'
  id: string
  command: string
  params: Record<string, any>
}

interface InvokeResponse {
  type: 'invoke-result'
  id: string
  result?: any
  error?: string
}

interface StatusUpdate {
  type: 'status-update'
  data: {
    proxyStatus: { running: boolean; socket_path?: string }
    socketAlive: boolean
    namespaceStatuses: Array<{ namespace: string; connected: boolean; tool_count: number }>
  }
}

type WorkerIncoming = 
  | { type: 'start'; intervalMs?: number }
  | { type: 'stop' }
  | InvokeResponse

// State
let polling = false
let intervalHandle: ReturnType<typeof setInterval> | null = null
let pendingInvokes = new Map<string, { resolve: (v: any) => void; reject: (e: string) => void; timer: ReturnType<typeof setTimeout> }>()
let pollInProgress = false
let idCounter = 0

const POLL_INTERVAL_MS = 15000
const INVOKE_TIMEOUT_MS = 5000

function generateId(): string {
  return `poll-${++idCounter}-${Date.now()}`
}

/** Send an invoke request to main thread and wait for response */
function invokeOnMain(command: string, params: Record<string, any> = {}): Promise<any> {
  return new Promise((resolve, reject) => {
    const id = generateId()
    
    const timer = setTimeout(() => {
      pendingInvokes.delete(id)
      reject(`Timeout: ${command} did not respond within ${INVOKE_TIMEOUT_MS}ms`)
    }, INVOKE_TIMEOUT_MS)
    
    pendingInvokes.set(id, { resolve, reject, timer })
    
    const msg: InvokeRequest = { type: 'invoke', id, command, params }
    self.postMessage(msg)
  })
}

/** Run one poll cycle — sequential to avoid socket contention */
async function pollCycle() {
  if (pollInProgress) return // mutex: skip if previous still running
  pollInProgress = true
  
  const data: StatusUpdate['data'] = {
    proxyStatus: { running: false },
    socketAlive: false,
    namespaceStatuses: []
  }
  
  try {
    // 1. Proxy status
    try {
      data.proxyStatus = await invokeOnMain('get_proxy_status')
    } catch (e) {
      // Keep default
    }
    
    // 2. Socket alive
    try {
      const result = await invokeOnMain('check_socket_alive')
      data.socketAlive = result?.alive ?? false
    } catch (e) {
      data.socketAlive = false
    }
    
    // 3. Namespace statuses (only if socket is alive)
    if (data.socketAlive) {
      try {
        data.namespaceStatuses = await invokeOnMain('get_namespace_statuses')
      } catch (e) {
        // Keep empty
      }
    }
    
    // Post consolidated update
    const update: StatusUpdate = { type: 'status-update', data }
    self.postMessage(update)
  } finally {
    pollInProgress = false
  }
}

// Message handler
self.onmessage = (e: MessageEvent<WorkerIncoming>) => {
  const msg = e.data
  
  if (msg.type === 'start') {
    if (polling) return
    polling = true
    const interval = msg.intervalMs ?? POLL_INTERVAL_MS
    
    // Immediate first poll
    pollCycle()
    
    intervalHandle = setInterval(() => pollCycle(), interval)
  }
  else if (msg.type === 'stop') {
    polling = false
    if (intervalHandle) {
      clearInterval(intervalHandle)
      intervalHandle = null
    }
    // Cancel all pending invokes
    for (const [_id, pending] of pendingInvokes) {
      clearTimeout(pending.timer)
      pending.reject('Worker stopped')
    }
    pendingInvokes.clear()
    pollInProgress = false
  }
  else if (msg.type === 'invoke-result') {
    const pending = pendingInvokes.get(msg.id)
    if (pending) {
      clearTimeout(pending.timer)
      pendingInvokes.delete(msg.id)
      if (msg.error) {
        pending.reject(msg.error)
      } else {
        pending.resolve(msg.result)
      }
    }
  }
}
