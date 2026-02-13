export {}

/**
 * Unified Poller Worker
 *
 * Supports multiple polling registrations in one worker instance.
 * Each registration gets its own interval and callback ID.
 */

type CallbackId = 'status-poll'

interface RegisterMessage {
  type: 'register'
  registrationId: string
  intervalMs: number
  callbackId: CallbackId
  immediate?: boolean
}

interface UnregisterMessage {
  type: 'unregister'
  registrationId: string
}

interface TriggerMessage {
  type: 'trigger'
  registrationId: string
}

interface StopAllMessage {
  type: 'stop-all'
}

interface InvokeResponseMessage {
  type: 'invoke-result'
  requestId: string
  result?: unknown
  error?: string
}

type WorkerIncoming = RegisterMessage | UnregisterMessage | TriggerMessage | StopAllMessage | InvokeResponseMessage

interface PollResultMessage {
  type: 'poll-result'
  registrationId: string
  callbackId: CallbackId
  payload: unknown
}

interface PollErrorMessage {
  type: 'poll-error'
  registrationId: string
  callbackId: CallbackId
  error: string
}

interface InvokeRequestMessage {
  type: 'invoke'
  requestId: string
  command: string
  params: Record<string, unknown>
}

interface Registration {
  registrationId: string
  callbackId: CallbackId
  intervalMs: number
  intervalHandle: ReturnType<typeof setInterval>
  inFlight: boolean
}

interface ProxyStatus {
  running: boolean
  socket_path?: string
}

interface NamespaceStatus {
  namespace: string
  connected: boolean
  tool_count: number
}

interface SocketStatus {
  alive: boolean
}

const registrations = new Map<string, Registration>()
const pendingInvokes = new Map<
  string,
  {
    resolve: (value: unknown) => void
    reject: (reason: string) => void
    timeoutHandle: ReturnType<typeof setTimeout>
  }
>()

let invokeCounter = 0
const INVOKE_TIMEOUT_MS = 5000

function nextInvokeId() {
  invokeCounter += 1
  return `invoke-${invokeCounter}-${Date.now()}`
}

function invokeOnMain(command: string, params: Record<string, unknown> = {}): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const requestId = nextInvokeId()
    const timeoutHandle = setTimeout(() => {
      pendingInvokes.delete(requestId)
      reject(`Timeout: ${command} exceeded ${INVOKE_TIMEOUT_MS}ms`)
    }, INVOKE_TIMEOUT_MS)

    pendingInvokes.set(requestId, { resolve, reject, timeoutHandle })

    const msg: InvokeRequestMessage = {
      type: 'invoke',
      requestId,
      command,
      params,
    }
    self.postMessage(msg)
  })
}

async function runStatusPollCallback(): Promise<{ proxyStatus: ProxyStatus; socketAlive: boolean; namespaceStatuses: NamespaceStatus[] }> {
  let proxyStatus: ProxyStatus = { running: false }
  let socketAlive = false
  let namespaceStatuses: NamespaceStatus[] = []

  try {
    const result = await invokeOnMain('get_proxy_status')
    proxyStatus = (result as ProxyStatus) ?? { running: false }
  } catch {
    // keep default
  }

  try {
    const result = await invokeOnMain('check_socket_alive')
    socketAlive = ((result as SocketStatus | null)?.alive ?? false)
  } catch {
    socketAlive = false
  }

  if (socketAlive) {
    try {
      const result = await invokeOnMain('get_namespace_statuses')
      namespaceStatuses = Array.isArray(result) ? (result as NamespaceStatus[]) : []
    } catch {
      namespaceStatuses = []
    }
  }

  return { proxyStatus, socketAlive, namespaceStatuses }
}

async function executeRegistration(registration: Registration) {
  if (registration.inFlight) return
  registration.inFlight = true

  try {
    switch (registration.callbackId) {
      case 'status-poll': {
        const payload = await runStatusPollCallback()
        const msg: PollResultMessage = {
          type: 'poll-result',
          registrationId: registration.registrationId,
          callbackId: registration.callbackId,
          payload,
        }
        self.postMessage(msg)
        break
      }
      default: {
        const msg: PollErrorMessage = {
          type: 'poll-error',
          registrationId: registration.registrationId,
          callbackId: registration.callbackId,
          error: `Unsupported callback ID: ${String(registration.callbackId)}`,
        }
        self.postMessage(msg)
      }
    }
  } catch (error) {
    const msg: PollErrorMessage = {
      type: 'poll-error',
      registrationId: registration.registrationId,
      callbackId: registration.callbackId,
      error: error instanceof Error ? error.message : String(error),
    }
    self.postMessage(msg)
  } finally {
    registration.inFlight = false
  }
}

function unregister(registrationId: string) {
  const existing = registrations.get(registrationId)
  if (!existing) return
  clearInterval(existing.intervalHandle)
  registrations.delete(registrationId)
}

function stopAll() {
  for (const registration of registrations.values()) {
    clearInterval(registration.intervalHandle)
  }
  registrations.clear()

  for (const pending of pendingInvokes.values()) {
    clearTimeout(pending.timeoutHandle)
    pending.reject('Worker stopped')
  }
  pendingInvokes.clear()
}

self.onmessage = (event: MessageEvent<WorkerIncoming>) => {
  const msg = event.data

  if (msg.type === 'register') {
    unregister(msg.registrationId)

    const registration: Registration = {
      registrationId: msg.registrationId,
      callbackId: msg.callbackId,
      intervalMs: msg.intervalMs,
      intervalHandle: setInterval(() => {
        void executeRegistration(registration)
      }, msg.intervalMs),
      inFlight: false,
    }

    registrations.set(msg.registrationId, registration)

    if (msg.immediate !== false) {
      void executeRegistration(registration)
    }
    return
  }

  if (msg.type === 'unregister') {
    unregister(msg.registrationId)
    return
  }

  if (msg.type === 'trigger') {
    const registration = registrations.get(msg.registrationId)
    if (registration) {
      void executeRegistration(registration)
    }
    return
  }

  if (msg.type === 'stop-all') {
    stopAll()
    return
  }

  if (msg.type === 'invoke-result') {
    const pending = pendingInvokes.get(msg.requestId)
    if (!pending) return

    clearTimeout(pending.timeoutHandle)
    pendingInvokes.delete(msg.requestId)

    if (msg.error) {
      pending.reject(msg.error)
    } else {
      pending.resolve(msg.result)
    }
  }
}

self.onclose = () => {
  stopAll()
}
